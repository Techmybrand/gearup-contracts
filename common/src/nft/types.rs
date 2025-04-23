use soroban_sdk::{contracterror, contracttype, symbol_short, Address, String, Symbol};

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
    StateNotAlreadySet = 11
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    ContractName,                    // Name of the NFT collection
    ContractSymbol,                  // Symbol of the NFT collection
    TokenMetadata(u64),              // Metadata for each token ID
    TokenOwnership(u64),             // Map of owners to their share amounts
    TemporaryControl(u64),           // Temporary control for rentals (renter, end_time)
}

#[derive(Clone)]
#[contracttype]
pub struct Token {
    pub id: u64,
    pub owner: Address,
    pub metadata: String,
}

#[contracttype]
pub struct TokenMetadata {
    pub total_shares: u32,
    pub token_uri: String
}

pub const ADMIN: Symbol = symbol_short!("ADMIN");
pub const MARKETPLACE_CONTRACT: Symbol = symbol_short!("MAR_CA");
