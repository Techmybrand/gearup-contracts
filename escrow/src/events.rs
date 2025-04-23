use soroban_sdk::{Address, Env, IntoVal, Val, Vec};

pub enum EscrowEvent {
    Initialized,
    Upgraded(u32),
    FundsLocked(u64, Address, Address, Address, i128),
    FundsReleased(u64, Address, i128),
    Refunded(u64, Address, i128),
}

impl EscrowEvent {
    pub fn name(&self) -> &'static str {
        match self {
            EscrowEvent::Initialized => stringify!(Initialized),
            EscrowEvent::Upgraded(..) => stringify!(Upgraded),
            EscrowEvent::FundsLocked(..) => stringify!(FundsLocked),
            EscrowEvent::FundsReleased(..) => stringify!(FundsReleased),
            EscrowEvent::Refunded(..) => stringify!(Refunded),
        }
    }

    pub fn publish(&self, env: &Env) {
        let mut v: Vec<Val> = Vec::new(&env);

        match self {
            EscrowEvent::Initialized => {}
            EscrowEvent::Upgraded(version) => {
                v.push_back(version.into_val(env));
            }
            EscrowEvent::FundsLocked(listing_id, seller, buyer, token, amount) => {
                v.push_back(listing_id.into_val(env));
                v.push_back(seller.into_val(env));
                v.push_back(buyer.into_val(env));
                v.push_back(token.into_val(env));
                v.push_back(amount.into_val(env));
            }
            EscrowEvent::FundsReleased(listing_id, seller, amount) => {
                v.push_back(listing_id.into_val(env));
                v.push_back(seller.into_val(env));
                v.push_back(amount.into_val(env));
            }
            EscrowEvent::Refunded(listing_id, buyer, amount) => {
                v.push_back(listing_id.into_val(env));
                v.push_back(buyer.into_val(env));
                v.push_back(amount.into_val(env));
            }
        }

        env.events().publish((self.name(),), v)
    }
}
