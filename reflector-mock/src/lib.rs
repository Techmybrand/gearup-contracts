#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Asset {
    Stellar(Address),
    Other(Symbol),
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}

#[contract]
pub struct MockPriceOracleContract;

#[contractimpl]
impl MockPriceOracleContract {
    pub fn lastprice(_e: Env, _asset: Asset) -> Option<PriceData> {
        Some(PriceData {
            price: 1,
            timestamp: 1,
        })
    }

    pub fn decimals(_e: Env) -> u32 {
        14
    }
}