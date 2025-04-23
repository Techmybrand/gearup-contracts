use soroban_sdk::{contractclient, Address, BytesN, Env, Symbol, Vec};
use super::types::{Agreement, AgreementStatus, Error};

#[contractclient(name = "AgreementContractClient")]
pub trait AgreementContractTrait {
    fn initialize(env: Env, admin: Address, marketplace_contract_id: Address) -> Result<(), Error>;
    fn version() -> u32;
    fn upgrade(env: Env, new_wasm_hash: BytesN<32>);
    fn update_state(env: Env, state_key: Symbol, state_value: Address) -> Result<(), Error>;
    fn create_agreement(
        env: Env,
        listing_id: u64,
        user: Address,
        owner: Address,
        shares: u32,
        is_rental: bool,
        duration: u64,
    ) -> u64;
    fn get_agreement(env: Env, agreement_id: u64) -> Result<Agreement, Error>;
    fn get_user_agreements(env: Env, user: Address) -> Vec<u64>;
    fn get_listing_agreements(env: Env, listing_id: u64) -> Vec<u64>;
    fn get_agreement_status(env: Env, agreement_id: u64) -> Result<AgreementStatus, Error>;
    fn owner_fulfilled(env: Env, agreement_id: u64) -> Result<bool, Error>;
    fn complete_agreement(env: Env, agreement_id: u64, user: Address) -> Result<bool, Error>;
    fn terminate_agreement(env: Env, agreement_id: u64, terminator: Address)
        -> Result<bool, Error>;
}
