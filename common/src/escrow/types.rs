use soroban_sdk::{contracterror, contracttype, Address};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EscrowError {
    StateNotAlreadySet = 1,
    EscrowNotActive = 2,
    EscrowNotFound = 3,
    AlreadyInitialized = 4,
}

#[derive(Clone)]
#[contracttype]
pub struct Escrow {
    pub amount: i128,
    pub token: Address,
    pub buyer: Address,
    pub seller: Address,
    pub status: EscrowStatus,
}

#[derive(Clone)]
#[contracttype]
pub enum EscrowStatus {
    Active,
    Completed,
    Refunded,
}

#[derive(Clone)]
#[contracttype]
pub enum EscrowDataKey {
    Escrow(u64), // Escrow struct mapping
}
