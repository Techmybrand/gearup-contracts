use crate::storage::get_data;
use common::nft::types::MARKETPLACE_CONTRACT;
use soroban_sdk::{Address, Env};

pub fn require_marketplace_call(env: &Env) {
    let marketplace_address: Address = get_data(env, &MARKETPLACE_CONTRACT).unwrap();
    marketplace_address.require_auth();
}
