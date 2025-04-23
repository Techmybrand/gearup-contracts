#![cfg(test)]

use super::MarketplaceTest;
use crate::types::{Listing, ListingStatus};
use soroban_sdk::testutils::{Events, Ledger};
use soroban_sdk::{log, Env, String};

#[test]
pub fn test_create_listing() {
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

    log!(&test.env, "{}", test.env.events().all());

    let amount = test.marketplace_client.get_usdc_price();
    log!(&test.env, "XLM amount {}", amount);

    // Check NewListing event
    // let event_expected = (
    //     test.marketplace_client.address.clone(),
    //     (MarketplaceEvent::NewListing(
    //         listing_id.clone(),
    //         listing_id.clone(),
    //         reference_id.clone(),
    //     )
    //     .name(),)
    //         .into_val(&test.env),
    //     (listing_id, listing_id, reference_id.clone()).into_val(&test.env),
    // );

    // assert!(
    //     test.env.events().all().contains(event_expected),
    //     "new listing event not present"
    // );

    // Verify listing
    let listing: Listing = test.marketplace_client.get_listing(&listing_id);
    assert_eq!(listing.creator, test.alice);
    assert_eq!(listing.reference_id, reference_id);
    assert_eq!(listing.status, ListingStatus::Available);
    assert!(listing.duration > test.env.ledger().timestamp());
    // adjust timestamp
    test.env.ledger().set_timestamp(duration);

    let listing2: Listing = test.marketplace_client.get_listing(&listing_id);
    assert!(
        listing2.duration <= test.env.ledger().timestamp(),
        "Expected {} to be less than current timestamp",
        listing.duration,
    );
}

fn get_one_hour_duration(env: &Env) -> u64 {
    let current_ledger_time: u64 = env.ledger().timestamp(); // Get the current ledger time in seconds
    let one_hour_in_seconds: u64 = 3600; // 1 hour in seconds
    current_ledger_time + one_hour_in_seconds
}
