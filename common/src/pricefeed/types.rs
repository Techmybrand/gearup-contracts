use soroban_sdk::{contracterror, contracttype, Address, Vec};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum OracleError {
    AlreadyInitialized = 1,
    Unauthorized = 2,
    InvalidPrice = 3,
    MinimumUpdateInterval = 4,
    MaximumUpdateInterval = 5,
    StalePrice = 6,
    NotInitialized = 7,
    StateNotAlreadySet = 8
}

#[derive(Clone)]
#[contracttype]
pub struct PriceData {
    pub rate: i128,
    pub timestamp: u64,
    pub valid_period: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct OracleConfig {
    pub admin: Address,
    pub updaters: Vec<Address>,
    pub min_update_interval: u64,
    pub max_price_change: i128,
}