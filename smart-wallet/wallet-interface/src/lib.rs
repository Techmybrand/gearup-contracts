#![no_std]

use soroban_sdk::{contractclient, Address, BytesN, Env, Val, Vec};
use types::Error;
use user_op::UserOperation;

pub mod types;
pub mod user_op;

#[contractclient(name = "SmartWalletClient")]
pub trait SmartWalletInterface {
    fn __constructor(env: Env, owner_public_key: BytesN<32>) -> Result<(), Error>;
    fn version() -> u32;
    fn upgrade(env: Env, new_wasm_hash: BytesN<32>);
    fn update_signature_threshold(env: Env, threshold: u32) -> Result<(), Error>;
    fn add_signer(env: Env, signer: BytesN<32>, weight: u32) -> Result<(), Error>;
    fn remove_signer(env: Env, signer: BytesN<32>) -> Result<(), Error>;
    fn get_signers(env: Env) -> Vec<BytesN<32>>;
    fn get_nonce(env: &Env) -> u64;
    fn validate_op(env: Env, operation: UserOperation) -> Result<(), Error>;
    fn execute_op(env: Env, operation: UserOperation) -> Result<Val, Error>;
}

#[contractclient(name = "PayMasterClient")]
pub trait PayMasterInterface {
    fn __constructor(env: Env, admin: Address, gas_token: Address) -> Result<(), Error>;
    fn version() -> u32;
    fn upgrade(env: Env, new_wasm_hash: BytesN<32>);
    fn can_sponsor(env: &Env, account: Address, contract: Address, gas_estimate: i128) -> bool;
    fn deposit(env: &Env, from: Address, amount: i128) -> Result<(), Error>;
    fn add_sponsored_account(env: &Env, account: Address, daily_limit: i128) -> Result<(), Error>;
    fn add_sponsored_contracts(env: &Env, contracts: Vec<Address>) -> Result<(), Error>;
    fn record_gas_usage(env: &Env, account: Address, gas_used: i128) -> Result<(), Error>;
    fn get_deposit_balance(env: &Env, user: Address) -> i128;
    fn withdraw(env: &Env, user: Address, amount: i128) -> Result<(), Error>;
    fn get_remaining_daily_limit(env: Env, account: Address) -> u32;
}
