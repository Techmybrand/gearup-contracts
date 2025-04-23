use soroban_sdk::{contracterror, contracttype, symbol_short, Address, String, Symbol};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    ListingNotFound = 2,
    ListingNotAvailable = 3,
    ListingTypeMismatch = 4,
    InvalidNftOwner = 5,
    MissingMarketplaceContractId = 6,
    AgreementNotFound = 7,
    AgreementNotActive = 8,
    AgreementNotOwnedByCaller = 9,
    StateNotAlreadySet = 11,
    InsufficientBalance = 12,
    InsufficientShares = 13,
    InsufficientSharesForPurchase = 14,
    CurrencyNotSupported = 15,
    CannotModifyShareStructure = 16,
    InvalidSharesDistribution = 17
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Listing {
    pub id: u64,
    pub creator: Address,
    pub reference_id: String,
    pub metadata_uri: String,
    pub price: i128,
    pub duration: u64, // Duration is 0 for purchases
    pub allow_purchase: bool,
    pub allow_rent: bool,
    pub status: ListingStatus,
    pub total_shares: u32,
    pub reserved_shares: u32,
    pub available_shares: u32,
    pub agreement_id: u64,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
#[contracttype]
pub enum AssetType {
    Gear = 1,
    Courses = 2,
    Studios = 3
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
#[contracttype]
pub enum PurchaseType {
    Rent = 1,
    Buy = 2,
}

#[contracttype]
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum ListingStatus {
    Available = 1,
    Rented = 2,
    Leased = 3,
    Purchased = 4,
    Unavailable = 5,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Listing(u64),
    ListingCount,
    UserListings(Address),     // Listings owned by user
    OwnershipShares(u64),      // Map of owners to their ownership percentages for a listing
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Currency {
    NGNG,
    USDC,
    XLM,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Owner {
    pub owner_id: Address,
    pub share: u64
}

pub const ADMIN: Symbol = symbol_short!("ADMIN");
pub const NFT_CONTRACT: Symbol = symbol_short!("NFT_CA");
pub const AGREEMENT_CONTRACT: Symbol = symbol_short!("RAGR_CA");
pub const ESCROW_CONTRACT: Symbol = symbol_short!("ESCROW_CA");
pub const PRICE_FEED_CONTRACT: Symbol = symbol_short!("P_FEED_CA");
pub const REFLECTOR_ORACLE: Symbol = symbol_short!("REFLECTOR");
pub const PAYMENT_TOKEN: Symbol = symbol_short!("PAY_TOKEN");
pub const CURRENCY: Symbol = symbol_short!("CURRENCY");