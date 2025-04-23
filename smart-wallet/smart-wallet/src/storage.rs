use soroban_sdk::{contracttype, BytesN, Env, IntoVal, TryFromVal, Val};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Signers,            // Map of public key for signature verification
    SignatureThreshold, // Threshold for minimum weight from signers required
    TransactionNonce,   // Nonce to prevent replay attacks
}

#[contracttype]
#[derive(Clone)]
pub struct Signature {
    pub public_key: BytesN<32>,
    pub signature: BytesN<64>,
}

#[contracttype(export = false)]
#[derive(Clone, Debug, PartialEq)]
pub enum StorageType {
    Persistent,
    Instance,
}

pub struct Storage;

#[allow(unused)]
impl Storage {
    /// Set a value in storage
    pub fn set<K: IntoVal<Env, Val>, T: IntoVal<Env, Val>>(
        env: &Env,
        key: K,
        value: T,
        storage: StorageType,
    ) {
        match storage {
            StorageType::Persistent => {
                env.storage()
                    .persistent()
                    .set(&key.into_val(env), &value.into_val(env));
            }
            StorageType::Instance => {
                env.storage()
                    .instance()
                    .set(&key.into_val(env), &value.into_val(env));
            }
        }
    }

    /// Get a value from storage, returns `None` if not found
    pub fn get<K: IntoVal<Env, Val>, T: TryFromVal<Env, Val>>(
        env: &Env,
        key: K,
        storage: StorageType,
    ) -> T {
        match storage {
            StorageType::Persistent => env.storage().persistent().get(&key).unwrap(),
            StorageType::Instance => env.storage().instance().get(&key).unwrap(),
        }
    }

    pub fn get_or<K: IntoVal<Env, Val>, T: TryFromVal<Env, Val>>(
        env: &Env,
        key: K,
        storage: StorageType,
    ) -> Option<T> {
        match storage {
            StorageType::Persistent => env.storage().persistent().get(&key),
            StorageType::Instance => env.storage().instance().get(&key),
        }
    }

    /// Check if a key exists in storage
    pub fn remove<K: IntoVal<Env, Val>>(env: &Env, key: K, storage: StorageType) -> bool {
        match storage {
            StorageType::Persistent => env.storage().persistent().remove(&key),
            StorageType::Instance => env.storage().instance().remove(&key),
        }
        true
    }

    pub fn exists<K: IntoVal<Env, Val>>(env: &Env, key: K, storage: StorageType) -> bool {
        match storage {
            StorageType::Persistent => env.storage().persistent().has(&key),
            StorageType::Instance => env.storage().instance().has(&key),
        }
    }
}
