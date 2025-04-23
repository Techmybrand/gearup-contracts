use soroban_sdk::{contracttype, Env, IntoVal, TryFromVal, Val};

const WEEK_OF_LEDGERS: u32 = 60 * 60 * 24 / 5 * 7;

#[contracttype(export = false)]
#[derive(Clone, Debug, PartialEq)]
pub enum StorageType {
    Persistent,
    Instance,
    Temporary,
}

pub fn extend_instance(env: &Env) {
    let max_ttl = env.storage().max_ttl();
    env.storage()
            .instance()
            .extend_ttl(max_ttl - WEEK_OF_LEDGERS, max_ttl);
}

pub fn extend_persistent<K>(env: &Env, key: &K) where
K: IntoVal<Env, Val>, {
    let max_ttl: u32 = env.storage().max_ttl();

    env.storage().persistent().extend_ttl::<K>(
        key,
        max_ttl - WEEK_OF_LEDGERS,
        max_ttl,
    );
}

// PERSISTENT STORAGE
pub fn store_persistent<K, V>(env: &Env, key: &K, val: &V)
where
    K: IntoVal<Env, Val>,
    V: IntoVal<Env, Val>,
{
    env.storage().persistent().set(key, val);
    extend_persistent(env, key);
}

pub fn get_persistent<K, V>(env: &Env, key: &K) -> Option<V>
where
    K: IntoVal<Env, Val>,
    V: TryFromVal<Env, Val>,
{
    env.storage().persistent().get(key)
}

pub fn remove_persistent<K>(env: &Env, key: &K)
where
    K: IntoVal<Env, Val>,
{
    env.storage().persistent().remove(key)
}

pub fn has_persistent<K>(env: &Env, key: &K) -> bool
where
    K: IntoVal<Env, Val>,
{
    env.storage().persistent().has(key)
}

// INSTANCE STORAGE
pub fn store_data<K, V>(env: &Env, key: &K, val: &V)
where
    K: IntoVal<Env, Val>,
    V: IntoVal<Env, Val>,
{
    env.storage().instance().set(key, val);
    extend_instance(env);
}


pub fn get_data<K, V>(env: &Env, key: &K,) -> Option<V>
where
    K: IntoVal<Env, Val>,
    V: TryFromVal<Env, Val>,
{
    env.storage().instance().get(key)
}

pub fn remove_data<K>(env: &Env, key: &K)
where
    K: IntoVal<Env, Val>,
{
    env.storage().instance().remove(key)
}

pub fn has_data<K>(env: &Env, key: &K) -> bool
where
    K: IntoVal<Env, Val>,
{
    env.storage().instance().has(key)
}
