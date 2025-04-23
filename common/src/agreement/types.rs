use soroban_sdk::{contracterror, contracttype, symbol_short, Address, Symbol};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    ListingNotFound = 1,
    ListingNotAvailable = 2,
    ListingTypeMismatch = 3,
    AlreadyInitialized = 4,
    InvalidNftOwner = 5,
    MissingMarketplaceContractId = 6,
    AgreementNotFound = 7,
    AgreementNotActive = 8,
    AgreementNotOwnedByCaller = 9,
    AgreementIsAlreadyActive = 10,
    StateNotAlreadySet = 11
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Agreement(u64),
    AgreementCount,
    UserAgreements(Address),         // List of agreement IDs for a user
    ListingAgreements(u64),          // List of agreement IDs for a listing
}

#[derive(Clone)]
#[contracttype]
pub struct Agreement {
    pub id: u64,
    pub user: Address,
    pub owner: Address,
    pub listing_id: u64,
    pub timestamp: u64,
    pub shares: u32,           // Used for purchase agreements (ownership percentage)
    pub duration: Option<u64>, // Used for lease agreements
    pub end_time: Option<u64>,
    pub status: AgreementStatus,
    pub agreement_type: AgreementType
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
pub enum AgreementType {
    Lease,
    Purchase
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
pub enum AgreementStatus {
    Created = 1,
    Active = 2,
    Completed = 3,
    Terminated = 4,
    Paused = 5,
}

pub const ADMIN: Symbol = symbol_short!("ADMIN");
pub const MARKETPLACE_CONTRACT: Symbol = symbol_short!("MAR_CA");
