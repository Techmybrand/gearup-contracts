use soroban_sdk::{Address, Env, IntoVal, Val, Vec};

pub enum NFTEvent {
    Initialized,
    Upgraded(u32),
    Mint(u64, Address),
    Transfer(u64, Address, Address),
    TransferShares(u64, Address, Address, u32),
}

impl NFTEvent {
    pub fn name(&self) -> &'static str {
        match self {
            NFTEvent::Initialized => stringify!(Initialized),
            NFTEvent::Upgraded(..) => stringify!(Upgraded),
            NFTEvent::Mint(..) => stringify!(Mint),
            NFTEvent::Transfer(..) => stringify!(Transfer),
            NFTEvent::TransferShares(..) => stringify!(TransferSharesTransferShares),
        }
    }

    pub fn publish(&self, env: &Env) {
        let mut v: Vec<Val> = Vec::new(&env);

        match self {
            NFTEvent::Initialized => {}
            NFTEvent::Upgraded(version) => {
                v.push_back(version.into_val(env));
            }
            NFTEvent::Mint(token_id, owner) => {
                v.push_back(token_id.into_val(env));
                v.push_back(owner.into_val(env));
            }
            NFTEvent::Transfer(token_id, from, to) => {
                v.push_back(token_id.into_val(env));
                v.push_back(from.into_val(env));
                v.push_back(to.into_val(env));
            }
            NFTEvent::TransferShares(token_id, from, to, shares) => {
                v.push_back(token_id.into_val(env));
                v.push_back(from.into_val(env));
                v.push_back(to.into_val(env));
                v.push_back(shares.into_val(env));
            }
        }

        env.events().publish((self.name(),), v)
    }
}
