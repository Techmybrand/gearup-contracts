#![no_std]

mod events;
use common::escrow::{
    interface::EscrowContractTrait,
    types::{Escrow, EscrowDataKey as DataKey, EscrowError as Error, EscrowStatus},
};
use events::EscrowEvent;
use soroban_sdk::{contract, contractimpl, symbol_short, token, Address, BytesN, Env, Symbol};

pub const MARKETPLACE_CONTRACT: Symbol = symbol_short!("MAR_CA");
pub const ADMIN: Symbol = symbol_short!("ADMIN");

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContractTrait for EscrowContract {
    // Initialize escrow
    fn initialize(env: Env, admin: Address, marketplace_contract_id: Address) -> Result<(), Error> {
        admin.require_auth();
        if env.storage().instance().has::<Symbol>(&ADMIN) {
            return Err(Error::AlreadyInitialized);
        }

        env.storage().instance().set(&ADMIN, &admin);
        env.storage()
            .instance()
            .set(&MARKETPLACE_CONTRACT, &marketplace_contract_id);
        env.storage().instance().set(&ADMIN, &admin);
        EscrowEvent::Initialized.publish(&env);
        Ok(())
    }

    fn version() -> u32 {
        2
    }

    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();
        env.deployer().update_current_contract_wasm(new_wasm_hash);
        EscrowEvent::Upgraded(Self::version()).publish(&env);
    }

    fn update_state(env: Env, state_key: Symbol, state_value: Address) -> Result<(), Error> {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        if !env.storage().instance().has::<Symbol>(&state_key) {
            return Err(Error::StateNotAlreadySet);
        }

        env.storage().instance().set(&state_key, &state_value);
        env.events()
            .publish(("state_updated", state_key), state_value);

        Ok(())
    }

    fn get_escrow(env: Env, listing_id: u64) -> Result<Escrow, Error> {
        let escrow: Option<Escrow> = env
            .storage()
            .instance()
            .get::<_, Escrow>(&DataKey::Escrow(listing_id));
        if escrow.is_some() {
            Ok(escrow.unwrap())
        } else {
            Err(Error::EscrowNotFound)
        }
    }

    // Get the current status of the escrow
    fn status(env: Env, listing_id: u64) -> Result<EscrowStatus, Error> {
        let escrow: Escrow = Self::get_escrow(env, listing_id)?;
        Ok(escrow.status)
    }

    // Start a new escrow process
    // Funds will be locked indefinitely until the marketplace calls `release`.
    // Security of this escrow funds is therefore dependent on the marketplace
    // We may need to implement a timelock later on.
    fn lock_funds(
        env: Env,
        listing_id: u64,
        seller: Address,
        buyer: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), Error> {
        require_marketplace(&env);
        let escrow: Escrow = Escrow {
            amount,
            token: token.clone(),
            seller: seller.clone(),
            buyer: buyer.clone(),
            status: EscrowStatus::Active,
        };
        env.storage()
            .instance()
            .set(&DataKey::Escrow(listing_id), &escrow);
        EscrowEvent::FundsLocked(listing_id, seller, buyer, token, amount).publish(&env);
        Ok(())
    }

    // Release funds to the seller
    fn release(env: Env, listing_id: u64) -> Result<i128, Error> {
        let mut escrow: Escrow = Self::get_escrow(env.clone(), listing_id.clone())?;
        let mktplace_ca: Address = require_marketplace(&env);

        if !matches!(escrow.status, EscrowStatus::Active) {
            return Err(Error::EscrowNotActive);
        }

        // Since we're implementing multi-ownership, this transaction may have been owned by several parties.
        // We'll need to distribute the payment to all the shareholders based on share proportion.
        // We may seek a better and more gas effective ways to do this later but for now, we're using the marketplace contract as seller.
        // Marketplace will then handle distributing to co-owners
        let token_client: token::TokenClient<'_> = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &mktplace_ca, // &escrow.seller, 
            &escrow.amount,
        );

        escrow.status = EscrowStatus::Completed;
        env.storage()
            .instance()
            .set(&DataKey::Escrow(listing_id), &escrow);

        EscrowEvent::FundsReleased(listing_id, escrow.seller, escrow.amount).publish(&env);

        Ok(escrow.amount)
    }

    // Refund the buyer
    fn refund(env: Env, listing_id: u64) -> Result<(), Error> {
        require_marketplace(&env);
        let mut escrow: Escrow = Self::get_escrow(env.clone(), listing_id)?;

        assert!(
            matches!(escrow.status, EscrowStatus::Active),
            "Escrow is not active"
        );
        let token_client: token::TokenClient<'_> = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.buyer,
            &escrow.amount,
        );

        escrow.status = EscrowStatus::Refunded;
        env.storage()
            .instance()
            .set(&DataKey::Escrow(listing_id), &escrow);

        EscrowEvent::Refunded(listing_id, escrow.buyer, escrow.amount).publish(&env);

        Ok(())
    }

    // Implement a method for admin to withdraw escrow funds in case of emergencies 
}

fn require_marketplace(env: &Env) -> Address {
    let marketplace_address: Address = env.storage().instance().get(&MARKETPLACE_CONTRACT).unwrap();
    marketplace_address.require_auth();

    marketplace_address
}
