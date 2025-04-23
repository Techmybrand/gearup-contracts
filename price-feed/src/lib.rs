#![no_std]
use common::pricefeed::{
    interface::PriceOracleContractTrait,
    types::{OracleConfig, OracleError, PriceData},
};
use soroban_sdk::{contract, contractimpl, symbol_short, vec, Address, BytesN, Env, Symbol};

const ADMIN: Symbol = symbol_short!("ADMIN");
const PRICE: Symbol = symbol_short!("PRICE");
const CONFIG: Symbol = symbol_short!("CONFIG");

#[contract]
pub struct PriceOracleContract;

#[contractimpl]
impl PriceOracleContractTrait for PriceOracleContract {
    fn initialize(
        env: Env,
        admin: Address,
        initial_rate: i128,
        valid_period: u64,
        min_update_interval: u64,
        max_price_change: i128,
    ) -> Result<(), OracleError> {
        if env.storage().instance().has(&ADMIN) {
            return Err(OracleError::AlreadyInitialized);
        }

        let config = OracleConfig {
            admin: admin.clone(),
            updaters: vec![&env, admin.clone()],
            min_update_interval,
            max_price_change,
        };

        let price_data = PriceData {
            rate: initial_rate,
            timestamp: env.ledger().timestamp(),
            valid_period,
        };

        env.storage().instance().set(&CONFIG, &config);
        env.storage().instance().set(&PRICE, &price_data);

        env.events().publish(("initialized", admin), initial_rate);

        Ok(())
    }

    fn version() -> u32 {
        1
    }

    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    fn update_state(env: Env, state_key: Symbol, state_value: Address) -> Result<(), OracleError> {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        if !env.storage().instance().has::<Symbol>(&state_key) {
            return Err(OracleError::StateNotAlreadySet);
        }

        env.storage().instance().set(&state_key, &state_value);
        env.events()
            .publish(("state_updated", state_key), state_value);

        Ok(())
    }

    fn update_price(env: Env, updater: Address, new_rate: i128) -> Result<(), OracleError> {
        updater.require_auth();

        let config: OracleConfig = env
            .storage()
            .instance()
            .get(&CONFIG)
            .ok_or(OracleError::NotInitialized)?;

        if !config.updaters.contains(&updater) {
            return Err(OracleError::Unauthorized);
        }

        let current_price: PriceData = env
            .storage()
            .instance()
            .get(&PRICE)
            .ok_or(OracleError::NotInitialized)?;

        let current_time = env.ledger().timestamp();
        if current_time - current_price.timestamp < config.min_update_interval {
            return Err(OracleError::MinimumUpdateInterval);
        }

        let price_change = ((new_rate - current_price.rate) * 100) / current_price.rate;
        if price_change.abs() > config.max_price_change {
            return Err(OracleError::MaximumUpdateInterval);
        }

        let new_price_data = PriceData {
            rate: new_rate,
            timestamp: current_time,
            valid_period: current_price.valid_period,
        };

        env.storage()
            .instance()
            .set(&PRICE, &new_price_data);

        env.events().publish(("price_updated", updater), new_rate);

        Ok(())
    }

    fn get_price(env: Env) -> Result<(i128, u64), OracleError> {
        let price_data: PriceData = env
            .storage()
            .instance()
            .get(&PRICE)
            .ok_or(OracleError::NotInitialized)?;
        Ok((price_data.rate, price_data.timestamp))
    }

    fn add_updater(env: Env, admin: Address, new_updater: Address) -> Result<(), OracleError> {
        admin.require_auth();

        let mut config: OracleConfig = env
            .storage()
            .instance()
            .get(&CONFIG)
            .ok_or(OracleError::NotInitialized)?;

        if admin != config.admin {
            return Err(OracleError::Unauthorized);
        }

        if !config.updaters.contains(&new_updater) {
            config.updaters.push_back(new_updater.clone());
            env.storage()
                .instance()
                .set(&CONFIG, &config);

            env.events().publish(("updater_added", admin), new_updater);
        }

        Ok(())
    }

    fn remove_updater(env: Env, admin: Address, updater: Address) -> Result<(), OracleError> {
        admin.require_auth();

        let mut config: OracleConfig = env
            .storage()
            .instance()
            .get(&CONFIG)
            .ok_or(OracleError::NotInitialized)?;

        if admin != config.admin {
            return Err(OracleError::Unauthorized);
        }

        if let Some(index) = config.updaters.first_index_of(&updater) {
            config.updaters.remove(index);
            env.storage()
                .instance()
                .set(&CONFIG, &config);

            env.events().publish(("updater_removed", admin), updater);
        }

        Ok(())
    }

    fn update_config(
        env: Env,
        admin: Address,
        min_update_interval: u64,
        max_price_change: i128,
        valid_period: u64,
    ) -> Result<(), OracleError> {
        admin.require_auth();

        let mut config: OracleConfig = env
            .storage()
            .instance()
            .get(&CONFIG)
            .ok_or(OracleError::NotInitialized)?;

        if admin != config.admin {
            return Err(OracleError::Unauthorized);
        }

        config.min_update_interval = min_update_interval;
        config.max_price_change = max_price_change;

        let mut price_data: PriceData = env
            .storage()
            .instance()
            .get(&PRICE)
            .ok_or(OracleError::NotInitialized)?;

        price_data.valid_period = valid_period;

        env.storage()
            .instance()
            .set(&CONFIG, &config);
        env.storage()
            .instance()
            .set(&PRICE, &price_data);

        env.events()
            .publish(("config_updated", admin), min_update_interval);

        Ok(())
    }
}
