use common::escrow::interface::EscrowContractClient;
use soroban_sdk::{panic_with_error, token, Address, Env, Map, Symbol, Vec};

#[allow(unused)]
use crate::{
    oracle::oracle::{Asset, Client as OracleClient, PriceData},
    storage::{get_persistent, store_persistent},
    types::{DataKey, Error, Listing, ListingStatus},
};
use crate::{
    storage::get_data,
    types::{Currency, CURRENCY, PAYMENT_TOKEN},
};

#[allow(unused)]
use super::contract_clients::{
    get_agreement_client, get_escrow_client, get_feed_client, get_nft_client, get_oracle_client,
};

pub fn create_purchase_agreement(
    env: &Env,
    listing_id: &u64,
    renter_or_buyer: &Address,
    owner: &Address,
    duration: &u64,
    shares: &u32,
    is_rental: &bool,
) -> u64 {
    let agreement_id = get_agreement_client(&env).create_agreement(
        listing_id,
        renter_or_buyer,
        owner,
        shares,
        is_rental,
        duration,
    );

    agreement_id
}

pub fn complete_agreement(
    env: &Env,
    renter_or_buyer: Address,
    mut listing: Listing,
    is_rental: bool,
) {
    let agreement_client = get_agreement_client(env);
    let nft_client = get_nft_client(env);

    // Transfer NFT ownership
    if is_rental {
        // revoke temporary control
        listing.status = ListingStatus::Rented;
        agreement_client.owner_fulfilled(&listing.agreement_id);
    } else {
        nft_client.transfer(&listing.creator, &renter_or_buyer, &listing.id);
        listing.status = ListingStatus::Purchased;
        agreement_client.complete_agreement(&listing.agreement_id, &renter_or_buyer);
    }
}

#[allow(unused)]
pub fn terminate_agreement(
    env: &Env,
    agreement_id: &u64,
    listing_id: &u64,
    seller: &Address,
) -> bool {
    get_agreement_client(env).terminate_agreement(agreement_id, &seller);
    get_escrow_client(env).refund(listing_id);
    true
}

#[allow(unused)]
pub fn transfer_tokens_to_owner(
    env: &Env,
    amount: i128,
    from: &Address,
    to: &Address,
) {
    let token_addr: Address = get_data(env, &PAYMENT_TOKEN).unwrap();
    let token_client: token::Client<'_> = token::Client::new(&env, &token_addr);
    let balance: i128 = token_client.balance(from);
    let token_amount: i128 = parse_amount(&env, &amount);
    if balance < token_amount {
        panic_with_error!(&env, Error::InsufficientBalance)
    }
    token_client.transfer(from, &to, &token_amount);
}

pub fn transfer_and_lock_tokens(
    env: &Env,
    listing_id: u64,
    amount: i128,
    owner: &Address,
    from: &Address
) {
    let token_addr: Address = get_data(env, &PAYMENT_TOKEN).unwrap();
    let token_client: token::Client<'_> = token::Client::new(&env, &token_addr);

    let escrow_client: EscrowContractClient<'_> = get_escrow_client(&env);
    let escrow_contract: Address = escrow_client.address.clone();

    let balance: i128 = token_client.balance(from);

    let token_amount: i128 = parse_amount(&env, &amount);
    if balance < token_amount {
        panic_with_error!(&env, Error::InsufficientBalance)
    }
    token_client.transfer(from, &escrow_contract, &token_amount);
    escrow_client.lock_funds(&listing_id, owner, &from, &token_addr, &token_amount);
}

pub fn distribute_dividends(
    env: &Env,
    listing_id: u64,
    payment_amount: i128,
) -> Result<(), Error> {
    let ownership_shares: Map<Address, u32> = 
        get_persistent(&env, &DataKey::OwnershipShares(listing_id)).unwrap_or(Map::new(&env));
    let listing: Listing = get_listing_by_id(&env, listing_id);
    
    for (owner, shares) in ownership_shares.iter() {
        // Calculate proportional payment
        let owner_payment = (payment_amount * shares as i128) / listing.total_shares as i128;
        
        // Transfer payment to owner
        if owner_payment > 0 {
            transfer_tokens_to_owner(&env, owner_payment,  &env.current_contract_address(), &owner);
        }
    }
    
    Ok(())
}

pub fn get_listing_by_id(env: &Env, listing_id: u64) -> Listing {
    let listing: Option<Listing> = get_persistent(&env, &DataKey::Listing(listing_id));

    if listing.is_none() {
        panic_with_error!(&env, Error::ListingNotFound);
    }

    listing.unwrap()
}

pub fn parse_amount(env: &Env, amount: &i128) -> i128 {
    // This converts the GUPT to USDC but relevance is questionable since GUPT is also a stable currency.
    // Maybe handle offchain?
    // Disabled for now till we agree on a best approach, so just do a dereferenced forward of amount;

    let curr: Currency = get_data(env, &CURRENCY).unwrap();
    if curr == Currency::NGNG {
        *amount
    } else {
        let (usdc_rate, decimals) = get_usdc_price(env); // price of base in USD
        let (usd_rate, _) = get_feed_client(&env).get_price(); // with 7 decimal, pricee of quote in USD

        let usd_amount: i128 = (amount * 1_0_000_000) / usd_rate;
        let usdc_amount: i128 = (usd_amount * 10_i128.pow(decimals)) / usdc_rate;

        usdc_amount
    }
}

#[cfg(test)]
pub fn get_usdc_price(_env: &Env) -> (i128, u32) {
    (1_0_000_000, 7)
}

#[cfg(not(test))]
pub fn get_usdc_price(env: &Env) -> (i128, u32) {
    let reflector_oracle: OracleClient<'_> = get_oracle_client(&env);

    let asset: Asset = Asset::Other(Symbol::new(env, "USDC"));
    let rate: PriceData = reflector_oracle.lastprice(&asset).unwrap();
    let decimals: u32 = reflector_oracle.decimals();

    (rate.price, decimals)
}

pub fn remove_listing(env: &Env, listing_id: u64, owner: Address) {
    let mut from_listings: Vec<u64> = get_persistent(&env, &DataKey::UserListings(owner.clone()))
        .unwrap_or_else(|| Vec::new(&env));

    // Find and remove the listing
    let mut index: u32 = 0;

    while index < from_listings.len() {
        if from_listings.get(index).unwrap() == listing_id {
            from_listings.remove(index);
            store_persistent(&env, &DataKey::UserListings(owner), &from_listings);
            break;
        }
        index += 1;
    }
}

#[allow(unused)]
pub fn add_listing(env: &Env, listing_id: u64, owner: Address) {
    let mut listings: Vec<u64> = get_persistent(&env, &DataKey::UserListings(owner.clone()))
        .unwrap_or_else(|| Vec::new(&env));
    if !listings.iter().any(|id| id == listing_id) {
        listings.push_back(listing_id);
        store_persistent(&env, &DataKey::UserListings(owner), &listings);
    }
}
