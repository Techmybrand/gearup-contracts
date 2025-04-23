#![cfg(test)]
extern crate std;

use super::MarketplaceTest;
use crate::types::Listing;
use soroban_sdk::{log, Env, String};

#[test]
fn test_purchase_or_rent() {
    let test: MarketplaceTest = MarketplaceTest::setup();
    let duration: u64 = get_one_hour_duration(&test.env);
    let price: i128 = 1_000_0_000_000; //1_000 in 7 decimals
    let reference_id = String::from_str(&test.env, "acy23bza");
    let metadata_uri = String::from_str(
        &test.env,
        "https://gearup.market/listings/290zds9olashe9we0239jdo42jas",
    );

    let listing_id: u64 = test.marketplace_client.create_listing(
        &test.alice,
        &reference_id,
        &metadata_uri,
        &price,
        &duration,
        &true,
        &true,
        &1_000u32,
        &100u32
    );

    // Verify listing
    let listing: Listing = test.marketplace_client.get_listing(&listing_id);
    assert_eq!(listing.creator, test.alice);
    assert!(
        test.token_client.balance(&test.bob) > 0,
        "Token balance is not empty"
    );

    test.marketplace_client.rent(
        &listing_id,
        &test.bob,
        &price,
        &duration,
    );

    let listing2: Listing = test.marketplace_client.get_listing(&listing_id);
    log!(&test.env, "Status {}", listing2.status);
}

fn get_one_hour_duration(env: &Env) -> u64 {
    let current_ledger_time: u64 = env.ledger().timestamp(); // Get the current ledger time in seconds
    let one_hour_in_seconds: u64 = 3600; // 1 hour in seconds
    current_ledger_time + one_hour_in_seconds
}
