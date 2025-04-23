use soroban_sdk::{contractclient, Address, BytesN, Env, Symbol};

use super::types::OracleError;

#[contractclient(name = "PriceOracleContractClient")]
pub trait PriceOracleContractTrait {
    fn initialize(
        env: Env,
        admin: Address,
        initial_rate: i128,
        valid_period: u64,
        min_update_interval: u64,
        max_price_change: i128,
    ) -> Result<(), OracleError>;
    fn version() -> u32;
    fn upgrade(env: Env, new_wasm_hash: BytesN<32>);
    fn update_state(env: Env, state_key: Symbol, state_value: Address) -> Result<(), OracleError>;
    fn update_price(env: Env, updater: Address, new_rate: i128) -> Result<(), OracleError>;
    fn get_price(env: Env) -> Result<(i128, u64), OracleError>;
    fn add_updater(env: Env, admin: Address, new_updater: Address) -> Result<(), OracleError>;
    fn remove_updater(env: Env, admin: Address, updater: Address) -> Result<(), OracleError>;
    fn update_config(
        env: Env,
        admin: Address,
        min_update_interval: u64,
        max_price_change: i128,
        valid_period: u64,
    ) -> Result<(), OracleError>;
}
