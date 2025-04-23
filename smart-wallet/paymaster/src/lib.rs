#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, vec, Address, BytesN, Env, IntoVal, Symbol, Vec
};
use wallet_interface::{types::Error, PayMasterInterface};

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Admin,                      // Contract admin
    GasToken,                   // Token used for gas payments
    Deposit(Address),           // User deposits for gas
    SponsoredAccount(Address),  // Account daily limit
    SponsoredContract(Address), // User identity daily limit
    DailyUsage(Address),        // Track daily usage per account
    LastResetTime,              // Time when daily limits were last reset
}

#[contract]
pub struct Paymaster;

#[allow(unused)]
#[contractimpl]
impl PayMasterInterface for Paymaster {
    // Initialize the paymaster
    fn __constructor(env: Env, admin: Address, gas_token: Address) -> Result<(), Error> {
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::GasToken, &gas_token);
        env.storage()
            .instance()
            .set(&DataKey::LastResetTime, &env.ledger().timestamp());

        Ok(())
    }

    fn version() -> u32 {
        1
    }

    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        verify_admin(&env);
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    // Deposit tokens to cover gas for a user's operations
    fn deposit(env: &Env, from: Address, amount: i128) -> Result<(), Error> {
        from.require_auth();

        // Get the gas token
        let gas_token: Address = env.storage().instance().get(&DataKey::GasToken).unwrap();

        // Transfer tokens from user to this contract
        env.invoke_contract::<()>(
            &gas_token,
            &symbol_short!("transfer"),
            vec![
                env,
                from.clone().into_val(env),
                env.current_contract_address().into_val(env),
                amount.into_val(env),
            ],
        );

        // Update deposit balance
        let current_deposit = env
            .storage()
            .instance()
            .get(&DataKey::Deposit(from.clone()))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::Deposit(from), &(current_deposit + amount));

        Ok(())
    }

    // Add an account to sponsored list (admin only)
    fn add_sponsored_contracts(env: &Env, contracts: Vec<Address>) -> Result<(), Error> {
        verify_admin(&env)?;
        for contract in contracts.iter() {
            env.storage()
                .instance()
                .set(&DataKey::SponsoredContract(contract.clone()), &true);

            // Emit individual events for tracking
            env.events().publish(
                (
                    symbol_short!("paymaster"),
                    Symbol::new(&env, "add_contract"),
                ),
                contract,
            );
        }
        Ok(())
    }

    fn add_sponsored_account(env: &Env, account: Address, daily_limit: i128) -> Result<(), Error> {
        verify_admin(&env)?;
        env.storage()
            .instance()
            .set(&DataKey::SponsoredAccount(account), &daily_limit);

        Ok(())
    }

    // Check if an operation can be sponsored
    fn can_sponsor(env: &Env, account: Address, contract: Address, gas_estimate: i128) -> bool {
        // Reset daily limits if needed
        maybe_reset_daily_limits(&env);

        // Check if account is sponsored
        let sponsored_limit: Option<i128> = env
            .storage()
            .instance()
            .get(&DataKey::SponsoredAccount(account.clone()));

        if let Some(daily_limit) = sponsored_limit {
            let used: i128 = env
                .storage()
                .instance()
                .get(&DataKey::DailyUsage(account.clone()))
                .unwrap_or(0);
            if used + gas_estimate <= daily_limit {
                return true;
            }
        }

        // Check if user is sponsored
        if env
            .storage()
            .instance()
            .has(&DataKey::SponsoredContract(contract))
        {
            return true;
        }

        // Check if user has sufficient deposit
        let deposit = env
            .storage()
            .instance()
            .get(&DataKey::Deposit(account))
            .unwrap_or(0);
        deposit >= gas_estimate
    }

    fn record_gas_usage(env: &Env, account: Address, gas_used: i128) -> Result<(), Error> {
        verify_admin(&env)?;

        // Reset daily limits if needed
        maybe_reset_daily_limits(&env);

        // Check if account is sponsored
        if env
            .storage()
            .instance()
            .has(&DataKey::SponsoredAccount(account.clone()))
        {
            // Update daily usage
            let used = env
                .storage()
                .instance()
                .get(&DataKey::DailyUsage(account.clone()))
                .unwrap_or(0);
            env.storage()
                .instance()
                .set(&DataKey::DailyUsage(account), &(used + gas_used));
            return Ok(());
        }

        // Deduct from user's deposit
        let deposit = env
            .storage()
            .instance()
            .get(&DataKey::Deposit(account.clone()))
            .unwrap_or(0);
        if deposit < gas_used {
            env.storage()
                .instance()
                .set(&DataKey::Deposit(account), &0u32);
            return Ok(());
        }

        env.storage()
            .instance()
            .set(&DataKey::Deposit(account), &(deposit - gas_used));
        Ok(())
    }

    // Get a user's deposit balance
    fn get_deposit_balance(env: &Env, user: Address) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::Deposit(user))
            .unwrap_or(0)
    }

    // Withdraw deposit (user can withdraw their own deposits)
    fn withdraw(env: &Env, user: Address, amount: i128) -> Result<(), Error> {
        user.require_auth();

        // Check if user has sufficient deposit
        let deposit = env
            .storage()
            .instance()
            .get(&DataKey::Deposit(user.clone()))
            .unwrap_or(0);
        if deposit < amount {
            return Err(Error::InsufficientDeposit);
        }

        // Update deposit
        env.storage()
            .instance()
            .set(&DataKey::Deposit(user.clone()), &(deposit - amount));

        // Transfer tokens to user
        let gas_token: Address = env.storage().instance().get(&DataKey::GasToken).unwrap();
        // Transfer tokens from user to this contract
        env.invoke_contract::<()>(
            &gas_token,
            &symbol_short!("transfer"),
            vec![
                env,
                env.current_contract_address().into_val(env),
                user.clone().into_val(env),
                amount.into_val(env),
            ],
        );

        Ok(())
    }

    // Get remaining daily limit for a wallet
    fn get_remaining_daily_limit(env: Env, account: Address) -> u32 {
        let daily_limit = env
            .storage()
            .instance()
            .get::<DataKey, u32>(&DataKey::SponsoredAccount(account.clone()))
            .unwrap_or(0);
        let current_usage = env
            .storage()
            .instance()
            .get::<DataKey, u32>(&DataKey::DailyUsage(account))
            .unwrap_or(0);

        daily_limit.saturating_sub(current_usage)
    }
}

// Helper to maybe reset daily limits at the start of a new day
fn maybe_reset_daily_limits(env: &Env) {
    let last_reset: u64 = env
        .storage()
        .instance()
        .get(&DataKey::LastResetTime)
        .unwrap();
    let current_time = env.ledger().timestamp();

    // Reset daily usage if it's a new day (86400 seconds in a day)
    if current_time - last_reset >= 86400 {
        // We don't need to explicitly clear all daily usage entries
        // Instead, we'll just update the reset time and let entries be checked individually
        env.storage()
            .instance()
            .set(&DataKey::LastResetTime, &current_time);
    }
}

// Verify admin
fn verify_admin(env: &Env) -> Result<(), Error> {
    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    stored_admin.require_auth();
    Ok(())
}
