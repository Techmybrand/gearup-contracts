#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, vec, Address, BytesN, Env, Symbol, Val, Vec
};

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    WalletWasmHash, // WASM hash for deploying smart walleet contract
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Error {
    AlreadyInitialized = 1,
    Unauthorized = 4,
}

#[contract]
pub struct SmartWalletFactory;

const EVENT_TAG: Symbol = symbol_short!("swf_v1");

#[contractimpl]
impl SmartWalletFactory {
    pub fn __constructor(
        env: &Env,
        admin: Address,
        account_wasm_hash: BytesN<32>,
    ) -> Result<(), Error> {
        // Check if admin is already set - if we can get it, contract is already initialized
        if env.storage()
            .instance()
            .get::<_, Address>(&DataKey::Admin)
            .is_some()
        {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::WalletWasmHash, &account_wasm_hash);
        Ok(())
    }

    pub fn create_wallet(env: Env, salt: BytesN<32>, signer: BytesN<32>) -> Result<Address, Error> {
        let admin = env
            .storage()
            .instance()
            .get::<_, Address>(&DataKey::Admin)
            .unwrap();
        admin.require_auth();

        let wasm_hash = env
            .storage()
            .instance()
            .get::<_, BytesN<32>>(&DataKey::WalletWasmHash)
            .unwrap();
        // Use deploy_v2 to pass constructor arguments directly
        let wallet_address = env
            .deployer()
            .with_current_contract(salt)
            .deploy_v2(wasm_hash, (signer,));

        // Emit an event for this wallet creation
        env.events()
            .publish((EVENT_TAG, symbol_short!("create")), wallet_address.clone());

        Ok(wallet_address)
    }
}

pub struct SmartWalletClient {
    env: Env,
    address: Address,
}

impl SmartWalletClient {
    pub fn new(env: &Env, address: &Address) -> Self {
        Self {
            env: env.clone(),
            address: address.clone(),
        }
    }

    pub fn call(&self, signer: &BytesN<32>) {
        let args: Vec<Val> = vec![&self.env, signer.to_val()];
        self.env
            .invoke_contract::<Val>(&self.address, &symbol_short!("call"), args);
    }
}
