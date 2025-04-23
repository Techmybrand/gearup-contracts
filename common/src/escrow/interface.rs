use soroban_sdk::{contractclient, Address, BytesN, Env, Symbol};
use super::types::{EscrowError as Error, Escrow, EscrowStatus};

#[contractclient(name = "EscrowContractClient")]
pub trait EscrowContractTrait {
    fn initialize(env: Env, admin: Address, marketplace_contract_id: Address) -> Result<(), Error>;
    fn version() -> u32;
    fn upgrade(env: Env, new_wasm_hash: BytesN<32>);
    fn update_state(env: Env, state_key: Symbol, state_value: Address) -> Result<(), Error>;
    fn get_escrow(env: Env, listing_id: u64) -> Result<Escrow, Error>;
    fn status(env: Env, listing_id: u64) -> Result<EscrowStatus, Error>;
    fn lock_funds(
        env: Env,
        listing_id: u64,
        seller: Address,
        buyer: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), Error>;
    fn release(env: Env, listing_id: u64) -> Result<i128, Error>;
    fn refund(env: Env, listing_id: u64) -> Result<(), Error>;
}
