use soroban_sdk::{contractclient, Address, BytesN, Env, Map, String, Symbol, Vec};
use super::types::{Error, TokenMetadata};

#[contractclient(name = "NFTContractClient")]
pub trait NFTInterface {
    fn initialize(
        env: Env,
        admin: Address,
        marketplace_contract_id: Address,
    ) -> Result<(), Error>;
    fn version() -> u32;
    fn upgrade(env: Env, new_wasm_hash: BytesN<32>);
    fn update_state(env: Env, state_key: Symbol, state_value: Address) -> Result<(), Error>;
    fn symbol(env: Env) -> String;
    fn name(env: Env) -> String;
    fn mint(env: Env, owner: Address, token_id: u64, shares: u32, token_uri: String) -> u64;
    fn owners_of(env: Env, token_id: u64) -> Vec<Address>;
    fn transfer(env: Env, from: Address, to: Address, token_id: u64) -> bool;
    fn transfer_shares(env: Env, from: Address, to: Address, token_id: u64, shares: u32) -> bool;
    fn burn_shares(env: Env, owner: Address, token_id: u64, shares: u32) -> bool;
    fn is_sole_owner(env: Env, token_id: u64, address: Address) -> bool;
    fn merge_shares(env: Env, owner: Address, token_id: u64) -> u32;
    fn grant_temporary_control(env: Env, token_id: u64, renter: Address, end_time: u64);
    fn revoke_temporary_control(env: Env, token_id: u64, renter: Address);
    fn has_control(env: Env, token_id: u64, address: Address) -> bool;
    fn balance_of(env: Env, token_id: u64, owner: Address) -> u32;
    fn total_supply(env: Env, token_id: u64) -> u32;
    fn get_all_owners(env: Env, token_id: u64) -> Option<Map<Address, u32>>;
    fn token_uri(env: Env, token_id: u64) -> String;
    fn set_token_uri(env: Env, token_id: u64, uri: String);
    fn get_metadata(env: Env, token_id: u64) -> Option<TokenMetadata>;
    fn tokens_of_owner(env: Env, owner: Address) -> Vec<(u64, u32)>;
    fn exists(env: Env, token_id: u64) -> bool;
}