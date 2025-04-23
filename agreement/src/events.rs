use soroban_sdk::{Address, Env, IntoVal, Val, Vec};
use common::agreement::types::AgreementType;

pub enum AgreementEvent {
    Initialized,
    Upgraded(u32),
    Created(u64, u64, Address, AgreementType),
    Fulfilled(u64, u64, Address),
    Completed(u64, Address),
    Terminated(u64, Address),
}

impl AgreementEvent {
    pub fn name(&self) -> &'static str {
        match self {
            AgreementEvent::Initialized => stringify!(Initialized),
            AgreementEvent::Upgraded(..) => stringify!(Upgraded),
            AgreementEvent::Created(..) => stringify!(Created),
            AgreementEvent::Fulfilled(..) => stringify!(Fulfilled),
            AgreementEvent::Completed(..) => stringify!(Completed),
            AgreementEvent::Terminated(..) => stringify!(Terminated),
        }
    }

    pub fn publish(&self, env: &Env) {
        let mut v: Vec<Val> = Vec::new(&env);

        match self {
            AgreementEvent::Initialized => {}
            AgreementEvent::Upgraded(version) => {
                v.push_back(version.into_val(env));
            }
            AgreementEvent::Created(agreement_id, listing_id, buyer, agreement_type) => {
                v.push_back(agreement_id.into_val(env));
                v.push_back(listing_id.into_val(env));
                v.push_back(buyer.into_val(env));
                v.push_back(agreement_type.into_val(env));
            }
            AgreementEvent::Fulfilled(agreement_id, listing_id, owner) => {
                v.push_back(agreement_id.into_val(env));
                v.push_back(listing_id.into_val(env));
                v.push_back(owner.into_val(env));
            }
            AgreementEvent::Completed(listing_id, buyer) => {
                v.push_back(listing_id.into_val(env));
                v.push_back(buyer.into_val(env));
            }
            AgreementEvent::Terminated(listing_id, terminator) => {
                v.push_back(listing_id.into_val(env));
                v.push_back(terminator.into_val(env));
            }
        }

        env.events().publish((self.name(),), v)
    }
}
