#![no_std]

mod events;

use common::agreement::{
    interface::AgreementContractTrait,
    types::{
        Agreement, AgreementStatus, AgreementType, DataKey, Error, ADMIN, MARKETPLACE_CONTRACT,
    },
};
use events::AgreementEvent;
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Symbol, Vec};

pub fn require_marketplace_call(env: &Env) {
    let marketplace_address: Address = env.storage().instance().get(&MARKETPLACE_CONTRACT).unwrap();
    marketplace_address.require_auth();
}

#[contract]
pub struct AgreementContract;

#[contractimpl]
impl AgreementContractTrait for AgreementContract {
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
        env.storage()
            .instance()
            .set(&DataKey::AgreementCount, &0u64);
        AgreementEvent::Initialized.publish(&env);
        Ok(())
    }

    fn version() -> u32 {
        1
    }

    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();
        env.deployer().update_current_contract_wasm(new_wasm_hash);
        AgreementEvent::Upgraded(Self::version()).publish(&env);
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

    fn create_agreement(
        env: Env,
        listing_id: u64,
        user: Address,
        owner: Address,
        shares: u32,
        is_rental: bool,
        duration: u64,
    ) -> u64 {
        require_marketplace_call(&env);

        let current_time = env.ledger().timestamp();

        let agreement_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AgreementCount)
            .unwrap_or(0);
        let agreement_id: u64 = agreement_count + 1;

        let agreement_type: AgreementType = if is_rental {
            AgreementType::Lease
        } else {
            AgreementType::Purchase
        };

        let agreement: Agreement = match agreement_type {
            AgreementType::Purchase => Agreement {
                id: agreement_id,
                agreement_type: AgreementType::Purchase,
                user: user.clone(),
                owner: owner.clone(),
                listing_id,
                timestamp: env.ledger().timestamp(),
                shares,
                duration: None,
                end_time: None,
                status: AgreementStatus::Created,
            },
            AgreementType::Lease => Agreement {
                id: agreement_id,
                agreement_type: AgreementType::Lease,
                user: user.clone(),
                owner: owner.clone(),
                listing_id,
                timestamp: env.ledger().timestamp(),
                shares: 0, // Not applicable for lease
                duration: Some(duration),
                end_time: Some(duration + current_time),
                status: AgreementStatus::Created,
            },
        };

        env.storage()
            .instance()
            .set(&DataKey::Agreement(agreement_id), &agreement);

        // Update agreement count
        env.storage()
            .instance()
            .set(&DataKey::AgreementCount, &agreement_id);

        // Add to user's agreements
        let mut user_agreements: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::UserAgreements(user.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        user_agreements.push_back(agreement_id);
        env.storage()
            .instance()
            .set(&DataKey::UserAgreements(user.clone()), &user_agreements);

        // Add to listing's agreements
        let mut listing_agreements: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::ListingAgreements(listing_id))
            .unwrap_or_else(|| Vec::new(&env));
        listing_agreements.push_back(agreement_id);
        env.storage()
            .instance()
            .set(&DataKey::ListingAgreements(listing_id), &listing_agreements);

        AgreementEvent::Created(agreement_id, listing_id, user, agreement_type).publish(&env);

        agreement_id
    }

    fn get_agreement(env: Env, agreement_id: u64) -> Result<Agreement, Error> {
        let agreement: Option<Agreement> = env
            .storage()
            .instance()
            .get::<_, Agreement>(&DataKey::Agreement(agreement_id));
        if agreement.is_some() {
            Ok(agreement.unwrap())
        } else {
            Err(Error::AgreementNotFound)
        }
    }

    fn get_user_agreements(env: Env, user: Address) -> Vec<u64> {
        env.storage()
            .instance()
            .get(&DataKey::UserAgreements(user))
            .unwrap_or_else(|| Vec::new(&env))
    }

    // Get all agreements for a listing
    fn get_listing_agreements(env: Env, listing_id: u64) -> Vec<u64> {
        env.storage()
            .instance()
            .get(&DataKey::ListingAgreements(listing_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    fn get_agreement_status(env: Env, agreement_id: u64) -> Result<AgreementStatus, Error> {
        let agreement: Agreement = Self::get_agreement(env.clone(), agreement_id)?;
        Ok(agreement.status)
    }

    fn owner_fulfilled(env: Env, agreement_id: u64) -> Result<bool, Error> {
        require_marketplace_call(&env);
        let mut agreement: Agreement = Self::get_agreement(env.clone(), agreement_id)?;

        if agreement.status != AgreementStatus::Created {
            return Err(Error::AgreementNotActive);
        }

        agreement.status = AgreementStatus::Active;
        env.storage()
            .instance()
            .set(&DataKey::Agreement(agreement_id), &agreement);

        AgreementEvent::Fulfilled(agreement_id, agreement.listing_id, agreement.user).publish(&env);

        Ok(true)
    }

    fn complete_agreement(env: Env, agreement_id: u64, user: Address) -> Result<bool, Error> {
        require_marketplace_call(&env);
        let mut agreement: Agreement = Self::get_agreement(env.clone(), agreement_id)?;

        if agreement.user != user || agreement.owner != user {
            return Err(Error::AgreementNotOwnedByCaller);
        }

        if agreement.status != AgreementStatus::Created
            || agreement.status != AgreementStatus::Active
        {
            return Err(Error::AgreementNotActive);
        }

        agreement.status = AgreementStatus::Completed;
        env.storage()
            .instance()
            .set(&DataKey::Agreement(agreement_id), &agreement);

        AgreementEvent::Completed(agreement_id, user).publish(&env);

        Ok(true)
    }

    fn terminate_agreement(
        env: Env,
        agreement_id: u64,
        terminator: Address,
    ) -> Result<bool, Error> {
        terminator.require_auth();

        let mut agreement: Agreement = Self::get_agreement(env.clone(), agreement_id)?;
        if agreement.owner != terminator {
            return Err(Error::AgreementNotOwnedByCaller);
        }

        if agreement.status == AgreementStatus::Active {
            return Err(Error::AgreementIsAlreadyActive);
        }

        agreement.status = AgreementStatus::Terminated;
        env.storage()
            .instance()
            .set(&DataKey::Agreement(agreement_id), &agreement);

        AgreementEvent::Terminated(agreement_id, terminator).publish(&env);
        Ok(true)
    }
}
