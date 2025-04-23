#![no_std]
mod storage;

use soroban_sdk::{
    auth::{
        Context, ContractContext, CustomAccountInterface, InvokerContractAuthEntry,
        SubContractInvocation,
    },
    contract, contractimpl,
    crypto::Hash,
    symbol_short, vec, Address, BytesN, Env, Map, Symbol, Val, Vec,
};
use storage::{DataKey, Storage, StorageType};
use wallet_interface::{
    types::{Error, Signature},
    user_op::UserOperation,
    SmartWalletInterface,
};

#[contract]
pub struct SmartWallet;

const EVENT_TAG: Symbol = symbol_short!("sw_v1");

#[contractimpl]
impl SmartWalletInterface for SmartWallet {
    // Initialize the smart account with owner and signers
    fn __constructor(env: Env, public_key: BytesN<32>) -> Result<(), Error> {
        // Prevent re-initialization
        if Storage::exists(&env, DataKey::SignatureThreshold, StorageType::Persistent) {
            return Err(Error::AlreadyInitialized);
        }

        // Store account metadata
        Storage::set(&env, DataKey::TransactionNonce, &0u64, StorageType::Instance);

        // Initialize multisig with single signer (weight 1)
        let mut signers = Map::new(&env);
        signers.set(public_key.clone(), 1u32);

        Storage::set(&env, DataKey::Signers, signers, StorageType::Persistent);
        Storage::set(&env, DataKey::SignatureThreshold, &1u32, StorageType::Persistent);

        env.events().publish(
            (EVENT_TAG, symbol_short!("init"), symbol_short!("add")),
            public_key,
        );
        Ok(())
    }

    fn version() -> u32 {
        1
    }

    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        get_account(&env).require_auth();
        env.deployer().update_current_contract_wasm(new_wasm_hash);
        env.events()
            .publish((EVENT_TAG, symbol_short!("upgrade")), Self::version());
    }

    fn update_signature_threshold(env: Env, threshold: u32) -> Result<(), Error> {
        get_account(&env).require_auth();

        let signers: Map<BytesN<32>, u32> = get_signers_map(&env);
        let mut total_weight: u32 = 0;
        let min_weight: u32 = Storage::get(&env, DataKey::SignatureThreshold, StorageType::Persistent);

        for (_, weight) in signers.iter() {
            total_weight = total_weight.saturating_add(weight);
        }
        if total_weight < min_weight {
            return Err(Error::InvalidSignatureThreshold);
        }

        Storage::set(&env, DataKey::SignatureThreshold, &threshold, StorageType::Persistent);
        Ok(())
    }

    // Add/update (weight) of an account's signer
    fn add_signer(env: Env, signer: BytesN<32>, weight: u32) -> Result<(), Error> {
        get_account(&env).require_auth();

        let mut signers: Map<BytesN<32>, u32> = get_signers_map(&env);
        let signer_exist: bool = signers.contains_key(signer.clone());
        signers.set(signer.clone(), weight);
        Storage::set(&env, DataKey::Signers, signers, StorageType::Persistent);

        if !signer_exist {
            env.events()
                .publish((EVENT_TAG, symbol_short!("add")), signer);
        } else {
            env.events()
                .publish((EVENT_TAG, symbol_short!("update")), signer);
        }
        Ok(())
    }

    // Remove a signer from the account
    fn remove_signer(env: Env, signer: BytesN<32>) -> Result<(), Error> {
        get_account(&env).require_auth();

        let mut signers: Map<BytesN<32>, u32> = get_signers_map(&env);
        if signers.len() == 1 {
            return Err(Error::AtLeastOneSignerRequired);
        }

        signers.remove(signer.clone());
        Storage::set(&env, DataKey::Signers, signers.clone(), StorageType::Persistent);

        let mut total_remaining_weight: u32 = 0;
        let min_weight: u32 = Storage::get(&env, DataKey::SignatureThreshold, StorageType::Persistent);

        for (_, weight) in signers.iter() {
            total_remaining_weight = total_remaining_weight.saturating_add(weight);
        }
        if total_remaining_weight < min_weight {
            Storage::set(&env, DataKey::SignatureThreshold, total_remaining_weight, StorageType::Persistent);
            env.events().publish(
                (EVENT_TAG, symbol_short!("upd_s_thr")),
                total_remaining_weight,
            );
        }

        env.events()
            .publish((EVENT_TAG, symbol_short!("remove")), signer);
        Ok(())
    }

    fn get_signers(env: Env) -> Vec<BytesN<32>> {
        let signers: Map<BytesN<32>, u32> = get_signers_map(&env);
        signers.keys()
    }
    

    fn get_nonce(env: &Env) -> u64 {
        get_tx_nonce(&env)
    }

    fn validate_op(env: Env, operation: UserOperation) -> Result<(), Error> {
        if operation.nonce != get_tx_nonce(&env) {
            return Err(Error::InvalidNonce);
        } else if operation.valid_until < env.ledger().timestamp() {
            return Err(Error::OperationExpired);
        }
        Ok(())
    }

    fn execute_op(env: Env, operation: UserOperation) -> Result<Val, Error> {
        Self::validate_op(env.clone(), operation.clone())?;
        // Self::get_account(&env).require_auth();
        authenticate(&env, &operation.hash(&env), &operation.signatures)?;
        increment_nonce(&env);

        // Authorize this contract to call the target contract
        env.authorize_as_current_contract(vec![
            &env,
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: operation.target_contract.clone(),
                    fn_name: operation.function.clone(),
                    args: operation.args.clone(),
                },
                sub_invocations: vec![&env],
            }),
        ]);

        // Execute the operation
        let result: Val = env.invoke_contract::<Val>(
            &operation.target_contract.clone(),
            &operation.function.clone(),
            operation.args.clone(),
        );

        Ok(result)
    }
}

#[contractimpl]
impl CustomAccountInterface for SmartWallet {
    type Signature = Vec<Signature>;
    type Error = Error;

    #[allow(non_snake_case)]
    fn __check_auth(
        env: Env,
        signature_payload: Hash<32>,
        signatures: Self::Signature,
        auth_context: Vec<Context>,
    ) -> Result<(), Error> {
        // Perform authentication
        authenticate(&env, &signature_payload, &signatures)?;

        let tot_signers: u32 = get_signer_count(&env);
        let all_signed: bool = tot_signers == signatures.len();
        let curr_contract: Address = get_account(&env);

        // Verify authorization policy for each context
        for context in auth_context.iter() {
            verify_authorization_policy(&env, &context, &curr_contract, all_signed)?;
        }
        Ok(())
    }
}

// Authenticate signatures
fn authenticate(
    env: &Env,
    signature_payload: &Hash<32>,
    signatures: &Vec<Signature>,
) -> Result<(), Error> {
    // Track total weight of valid signatures
    let mut total_weight: u32 = 0u32;
    // Verify each signature
    let threshold: u32 = Storage::get::<_, u32>(env, DataKey::SignatureThreshold, StorageType::Persistent);

    for signature in signatures.iter() {
        // Check if signer is authorized
        let signers: Map<BytesN<32>, u32> = get_signers_map(env);
        let signer: Option<u32> = signers.get(signature.public_key.clone());
        if let Some(weight) = signer {
            // Verify Ed25519 signature
            env.crypto().ed25519_verify(
                &signature.public_key,
                &signature_payload.clone().into(),
                &signature.signature,
            );

            total_weight += weight;

            // Early return if threshold is met
            if total_weight >= threshold {
                return Ok(());
            }
        } else {
            return Err(Error::Unauthorized);
        }
    }

    // if we get here, verification was not successful
    return Err(Error::NotEnoughSigners);
}

// Verify authorization policy for operations
fn verify_authorization_policy(
    _: &Env,
    context: &Context,
    curr_contract: &Address,
    all_signed: bool,
) -> Result<(), Error> {
    // No limitations if all signers sign
    if all_signed {
        return Ok(());
    }

    match context {
        Context::Contract(c) => {
            // Prevent modifying this contract without full signatures
            if &c.contract == curr_contract {
                return Err(Error::NotEnoughSigners);
            }
            c
        }
        // Prevent contract creation without full signatures
        Context::CreateContractHostFn(_) | Context::CreateContractWithCtorHostFn(_) => {
            return Err(Error::NotEnoughSigners);
        }
    };

    Ok(())
}

// get account address
fn get_account(env: &Env) -> Address {
    env.current_contract_address()
}

// Get current nonce
fn get_signer_count(env: &Env) -> u32 {
    let signers: Map<BytesN<32>, u32> = Storage::get(&env, DataKey::Signers, StorageType::Persistent);
    signers.len()
}

fn get_signers_map(env: &Env) -> Map<BytesN<32>, u32> {
    Storage::get(env, DataKey::Signers, StorageType::Persistent)
}

// Get current nonce
fn get_tx_nonce(env: &Env) -> u64 {
    Storage::get(&env, DataKey::TransactionNonce, StorageType::Instance)
}

fn increment_nonce(env: &Env) -> u64 {
    let nonce: u64 = Storage::get_or::<_, u64>(&env, DataKey::TransactionNonce, StorageType::Instance).unwrap_or(0);
    let new_nonce: u64 = nonce + 1;
    Storage::set(&env, DataKey::TransactionNonce, &new_nonce, StorageType::Instance);
    new_nonce
}

#[cfg(test)]
mod test;
