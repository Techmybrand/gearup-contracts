#![cfg(test)]
extern crate std;

use ed25519_dalek::{Keypair, Signer};
use rand::thread_rng;
use smart_wallet_factory::{SmartWalletFactory, SmartWalletFactoryClient};
use wallet_interface::{types::Signature, user_op::UserOperation, SmartWalletClient};
use soroban_sdk::{
    contract, contractimpl, log, symbol_short,
    testutils::Address as _,
    token::{Client as TokenClient, StellarAssetClient},
    vec,
    xdr::{
        HashIdPreimage, HashIdPreimageSorobanAuthorization, InvokeContractArgs, Limits, ScAddress,
        ScSymbol, SorobanAddressCredentials, SorobanAuthorizationEntry, SorobanAuthorizedFunction,
        SorobanAuthorizedInvocation, SorobanCredentials, ToXdr, VecM, WriteXdr,
    },
    Address, Bytes, BytesN, Env, IntoVal, Symbol, Vec,
};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn transfer_tokens(env: Env, sac: Address, from: Address, to: Address, amount: i128) {
        from.require_auth();
        TokenClient::new(&env, &sac).transfer(&from, &to, &amount);
    }
}

mod wallet {
    use soroban_sdk::auth::Context;
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/smart_wallet.wasm"
    );
}

fn address_to_sc_address(address: &Address) -> ScAddress {
    address.try_into().unwrap()
}

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (TokenClient<'a>, StellarAssetClient<'a>) {
    let sac = e.register_stellar_asset_contract_v2(admin.clone());
    (
        TokenClient::new(e, &sac.address()),
        StellarAssetClient::new(e, &sac.address()),
    )
}

// Helper function to create ed25519 keypair and get public key as BytesN<32>
fn create_keypair(e: &Env) -> (Keypair, BytesN<32>) {
    let keypair: Keypair = Keypair::generate(&mut thread_rng());
    let pubkey: BytesN<32> = keypair.public.to_bytes().into_val(e);
    (keypair, pubkey)
}

fn signer_public_key(e: &Env, signer: &Keypair) -> BytesN<32> {
    signer.public.to_bytes().into_val(e)
}

fn sign(e: &Env, signer: &Keypair, payload: &BytesN<32>) -> Signature {
    Signature {
        public_key: signer_public_key(e, signer),
        signature: signer
            .sign(payload.to_array().as_slice())
            .to_bytes()
            .into_val(e),
    }
}

#[test]
fn test_wallet_with_factory() {
    let e = Env::default();
    let tx_amount_1 = 100_i128;
    let tx_amount_2 = 50_i128;

    // Setup token contract
    let admin: Address = Address::generate(&e);
    let (token_client, token_admin_client) = create_token_contract(&e, &admin.clone());
    let token_contract = token_client.address.clone();

    // Upload wallet contract first to get the WASM hash
    let wallet_wasm_hash: BytesN<32> = e.deployer().upload_contract_wasm(wallet::WASM);
    log!(&e, "Wallet wasm hash: {}", wallet_wasm_hash.clone());

    // Deploy smart wallet factory
    let factory_address: Address = e.register(SmartWalletFactory {}, (&admin, wallet_wasm_hash));
    let factory: SmartWalletFactoryClient<'_> = SmartWalletFactoryClient::new(&e, &factory_address);
    log!(&e, "Wallet factory address: {}", factory_address.clone());

    // Create keypairs for signing
    let (signer1_keypair, signer1_pubkey) = create_keypair(&e);
    let (signer2_keypair, signer2_pubkey) = create_keypair(&e);

    // Create a wallet using the factory
    let identity: Symbol = symbol_short!("akhils");
    let salt: BytesN<32> = e.crypto().sha256(&identity.to_xdr(&e)).into();
    let wallet_address: Address = factory
        .mock_all_auths()
        .create_wallet(&salt, &signer1_pubkey);
    let wallet: SmartWalletClient<'_> = SmartWalletClient::new(&e, &wallet_address);
    log!(&e, "Wallet address: {}", wallet_address.clone());

    // Test wallet initialized
    assert_eq!(wallet.get_nonce(), 0u64); // CBQLLJ7NGFWQLISXEHV2P5G6M3PGEMNYAVZ6QBRPKVU2PDGUEG3KCVQV

    // Add second signer to the wallet with auth
    wallet.mock_all_auths().add_signer(&signer2_pubkey, &2u32);
    wallet.mock_all_auths().update_signature_threshold(&2u32);

    let example_contract_address = e.register(ExampleContract, ());
    let example_contract_client = ExampleContractClient::new(&e, &example_contract_address);

    // Mint tokens to the wallet
    let token_amount = 10_000_000_i128;
    token_admin_client
        .mock_all_auths()
        .mint(&wallet_address, &token_amount);
    assert_eq!(token_client.balance(&wallet_address), token_amount);

    // Create recipient
    let recipient = Address::generate(&e);
    let transfer_amount = tx_amount_1;

    // Create transfer invocation
    let transfer_invocation: SorobanAuthorizedInvocation = SorobanAuthorizedInvocation {
        function: SorobanAuthorizedFunction::ContractFn(InvokeContractArgs {
            contract_address: token_contract.clone().try_into().unwrap(),
            function_name: ScSymbol::try_from("transfer").unwrap(),
            args: std::vec![
                wallet_address.clone().try_into().unwrap(),
                recipient.clone().try_into().unwrap(),
                transfer_amount.try_into().unwrap(),
            ]
            .try_into()
            .unwrap(),
        }),
        sub_invocations: VecM::default(),
    };

    let root_invocation = SorobanAuthorizedInvocation {
        function: SorobanAuthorizedFunction::ContractFn(InvokeContractArgs {
            contract_address: example_contract_address.clone().try_into().unwrap(),
            function_name: ScSymbol::try_from("transfer_tokens").unwrap(),
            args: std::vec![
                token_contract.clone().try_into().unwrap(),
                wallet_address.clone().try_into().unwrap(),
                recipient.clone().try_into().unwrap(),
                transfer_amount.try_into().unwrap(),
            ]
            .try_into()
            .unwrap(),
        }),
        sub_invocations: std::vec![
            transfer_invocation.clone(),
        ]
        .try_into()
        .unwrap(),
    };

    // Create authorization payload
    let nonce = 0;
    let signature_expiration_ledger = e.ledger().sequence() + 100;

    let payload = HashIdPreimage::SorobanAuthorization(HashIdPreimageSorobanAuthorization {
        network_id: e.ledger().network_id().to_array().into(),
        nonce,
        signature_expiration_ledger,
        invocation: root_invocation.clone(),
    });

    // Convert to bytes and hash
    let payload_bytes = payload.to_xdr(Limits::none()).unwrap();
    let payload_bytes = Bytes::from_slice(&e, payload_bytes.as_slice());
    let payload_hash = e.crypto().sha256(&payload_bytes);

    // Sign the payload
    let sig1 = signer1_keypair.sign(&payload_hash.to_array());
    let sig2 = signer2_keypair.sign(&payload_hash.to_array());

    // Create signatures for smart wallet
    let mut signatures_vec = Vec::new(&e);
    signatures_vec.push_back(Signature {
        public_key: signer1_pubkey.clone(),
        signature: BytesN::from_array(&e, &sig1.to_bytes()),
    });
    signatures_vec.push_back(Signature {
        public_key: signer2_pubkey.clone(),
        signature: BytesN::from_array(&e, &sig2.to_bytes()),
    });

    // Create auth entry for token transfer
    let auth = SorobanAuthorizationEntry {
        credentials: SorobanCredentials::Address(SorobanAddressCredentials {
            address: address_to_sc_address(&wallet_address),
            nonce,
            signature_expiration_ledger,
            signature: signatures_vec.try_into().unwrap(),
        }),
        root_invocation,
    };

    // Set authorization for transaction and execute the transfer
    example_contract_client.set_auths(&[auth]).transfer_tokens(&token_contract, &wallet_address, &recipient, &tx_amount_1);

    // Execute the transfer
    // token_client.transfer(&wallet_address, &recipient, &100);

    // Verify the transfer
    assert_eq!(token_client.balance(&wallet_address), token_amount - tx_amount_1);
    assert_eq!(token_client.balance(&recipient), tx_amount_1);

    // Test UserOperation
    let recipient2 = Address::generate(&e);

    // Prepare args for UserOperation
    let mut args = Vec::new(&e);
    args.push_back(wallet_address.into_val(&e));
    args.push_back(recipient2.into_val(&e));
    args.push_back(tx_amount_2.into_val(&e));

    // Create UserOperation
    let mut op: UserOperation = UserOperation {
        nonce: 0u64,
        target_contract: token_contract.clone(),
        function: Symbol::new(&e, "transfer"),
        args,
        valid_until: e.ledger().timestamp() + 1000,
        signatures: vec![&e],
    };

    // Hash the operation
    let op_hash = op.hash(&e);

    // add signatures
    op.signatures = vec![&e, sign(&e, &signer1_keypair, &op_hash.clone().into()), sign(&e, &signer2_keypair, &op_hash.clone().into())];

    // Execute the operation through wallet
    wallet.execute_op(&op);

    // // Verify the transfer was successful
    assert_eq!(token_client.balance(&wallet_address), token_amount - (tx_amount_1 + tx_amount_2));
    assert_eq!(token_client.balance(&recipient2), tx_amount_2);
}
