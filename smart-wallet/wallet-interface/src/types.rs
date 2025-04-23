use soroban_sdk::{contracterror, contracttype, BytesN};

#[contracterror(export = false)]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum Error {
    NotFound = 1,
    AlreadyInitialized = 2,
    SignerExpired = 3,
    InvalidSignatureThreshold = 4,
    InvalidNonce = 5,
    OperationExpired = 6,
    Unauthorized = 7,
    NotEnoughSigners = 8,
    AtLeastOneSignerRequired = 9,
    InsufficientDeposit = 10,
    NotSponsored = 11
}

#[contracttype(export = false)]
#[derive(Clone, Debug, PartialEq)]
pub struct Signature {
    pub public_key: BytesN<32>,
    pub signature: BytesN<64>,
}
