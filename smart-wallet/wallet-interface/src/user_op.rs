use soroban_sdk::{contracttype, crypto::Hash, xdr::ToXdr, Address, Bytes, BytesN, Env, Symbol, Val, Vec};

use crate::types::Signature;

// User operation struct
#[contracttype]
#[derive(Clone)]
pub struct UserOperation {
    pub nonce: u64,
    pub target_contract: Address,
    pub function: Symbol,
    pub args: Vec<Val>,
    pub signatures: Vec<Signature>, // Ed25519 signature
    pub valid_until: u64,      // Timestamp when operation expires
}

impl UserOperation {
    pub fn to_bytes(&self, env: &Env) -> Bytes {
        let mut data: Bytes = Bytes::new(&env);

        // Convert nonce to bytes
        // data.extend_from_slice(operation.function.to_array());
        data.extend_from_slice(&self.nonce.to_le_bytes());
        data.append(&self.target_contract.clone().to_xdr(&env));
        data.append(&self.function.clone().to_xdr(&env));
        data.append(&self.args.clone().to_xdr(&env));
        data.extend_from_slice(&self.valid_until.to_le_bytes());

        data
    }

    pub fn hash(&self, env: &Env) -> Hash<32> {
        env.crypto().sha256(&self.to_bytes(&env))
    }

    pub fn verify_signature(&self, env: &Env, owner_pubkey: &BytesN<32>, signature: &BytesN<64>) -> bool {
        env.crypto()
            .ed25519_verify(owner_pubkey, &self.hash(env).into(), signature);

        true
    }
}
