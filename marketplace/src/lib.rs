#![no_std]
#![allow(clippy::unused_unit)]

mod events;
mod oracle;
mod storage;
mod types;
mod utils;

use events::MarketplaceEvent;
use soroban_sdk::{
    contract, contractimpl, panic_with_error, Address, BytesN, Env, Map, String, Symbol, Vec,
};
use storage::{
    get_data, get_persistent, has_data, remove_persistent, store_data, store_persistent,
};
use types::{
    Currency, DataKey, Error, Listing, ListingStatus, PurchaseType, ADMIN, AGREEMENT_CONTRACT,
    CURRENCY, ESCROW_CONTRACT, NFT_CONTRACT, PAYMENT_TOKEN, PRICE_FEED_CONTRACT, REFLECTOR_ORACLE,
};
use utils::{
    contract_clients::{get_agreement_client, get_escrow_client, get_nft_client},
    helpers::{
        complete_agreement, create_purchase_agreement, distribute_dividends, get_listing_by_id,
        get_usdc_price, parse_amount, remove_listing, terminate_agreement,
        transfer_and_lock_tokens, transfer_tokens_to_owner,
    },
};

#[contract]
pub struct MarketplaceContract;

#[allow(dead_code)]
#[contractimpl]
impl MarketplaceContract {
    pub fn initialize(
        env: Env,
        admin: Address,
        nft_ca: Address,
        agreement_ca: Address,
        escrow_ca: Address,
        price_feed_ca: Address,
        reflector_ca: Address,
        payment_token: Address,
    ) -> Result<(), Error> {
        admin.require_auth();
        if has_data::<Symbol>(&env, &ADMIN) {
            return Err(Error::AlreadyInitialized);
        }
        store_data(&env, &ADMIN, &admin);
        store_data(&env, &DataKey::ListingCount, &0u64);
        store_data(&env, &NFT_CONTRACT, &nft_ca);
        store_data(&env, &AGREEMENT_CONTRACT, &agreement_ca);
        store_data(&env, &ESCROW_CONTRACT, &escrow_ca);
        store_data(&env, &PRICE_FEED_CONTRACT, &price_feed_ca);
        store_data(&env, &REFLECTOR_ORACLE, &reflector_ca);
        store_data(&env, &PAYMENT_TOKEN, &payment_token);
        store_data(&env, &CURRENCY, &Currency::NGNG);

        MarketplaceEvent::Initialized(nft_ca, agreement_ca, escrow_ca).publish(&env);
        Ok(())
    }

    pub fn version() -> u32 {
        5
    }

    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        let admin: Address = get_data(&env, &ADMIN).unwrap();
        admin.require_auth();
        env.deployer().update_current_contract_wasm(new_wasm_hash);
        MarketplaceEvent::Upgraded(Self::version()).publish(&env);
    }

    pub fn update_state(env: Env, state_key: Symbol, state_value: Address) {
        let admin: Address = get_data(&env, &ADMIN).unwrap();
        admin.require_auth();

        if !env.storage().instance().has::<Symbol>(&state_key) {
            panic_with_error!(&env, Error::StateNotAlreadySet);
        }

        store_data(&env, &state_key, &state_value);
        env.events()
            .publish(("state_updated", state_key), state_value);
    }

    pub fn set_payment_token(env: Env, token_addr: Address) {
        let admin: Address = get_data(&env, &ADMIN).unwrap();
        admin.require_auth();
        store_data(&env, &PAYMENT_TOKEN, &token_addr);
    }

    pub fn set_currency(env: Env, currency: Currency) {
        let admin: Address = get_data(&env, &ADMIN).unwrap();
        admin.require_auth();

        if currency == Currency::XLM {
            panic_with_error!(&env, Error::CurrencyNotSupported);
        }
        store_data(&env, &CURRENCY, &currency);
    }

    pub fn create_listing(
        env: Env,
        creator: Address,
        reference_id: String,
        metadata_uri: String,
        price: i128,
        duration: u64,
        allow_purchase: bool,
        allow_rent: bool,
        total_shares: u32,
        reserved_shares: u32,
    ) -> Result<u64, Error> {
        creator.require_auth();

        // Ensure reserved shares don't exceed total shares
        if reserved_shares > total_shares {
            panic_with_error!(&env, Error::InvalidSharesDistribution);
        }

        let listing_count: u64 = get_data(&env, &DataKey::ListingCount).unwrap_or(0);
        let listing_id: u64 = listing_count + 1;

        let listing: Listing = Listing {
            id: listing_id,
            creator: creator.clone(),
            duration,
            price,
            reference_id: reference_id.clone(),
            metadata_uri: metadata_uri.clone(),
            allow_purchase,
            allow_rent,
            total_shares, // total shares can be zero to disable multi-ownership
            reserved_shares,
            available_shares: total_shares - reserved_shares, // Initially all shares are available
            status: ListingStatus::Available,
            agreement_id: 0u64,
        };

        store_persistent(&env, &DataKey::Listing(listing_id), &listing);
        store_data(&env, &DataKey::ListingCount, &listing_id);

        let mut ownership_shares: Map<Address, u32> = Map::new(&env);
        ownership_shares.set(creator.clone(), total_shares); // Initial creator owns 100%
        store_persistent(
            &env,
            &DataKey::OwnershipShares(listing_id),
            &ownership_shares,
        );

        // Add to user's listings
        let mut user_listings: Vec<u64> =
            get_persistent(&env, &DataKey::UserListings(creator.clone()))
                .unwrap_or_else(|| Vec::new(&env));
        user_listings.push_back(listing_id);
        store_persistent(
            &env,
            &DataKey::UserListings(creator.clone()),
            &user_listings,
        );

        // Call NFT contract to mint a new token for this listing
        let nft_id: u64 =
            get_nft_client(&env).mint(&creator, &listing_id, &total_shares, &metadata_uri);

        MarketplaceEvent::NewListing(listing_id, nft_id, reference_id).publish(&env);

        Ok(listing_id)
    }

    pub fn add_listing_shares(
        env: Env,
        creator: Address,
        listing_id: u64,
        shares_to_add: u32,
        reserved_shares: u32,
    ) {
        creator.require_auth();

        let mut listing: Listing = get_listing_by_id(&env, listing_id);

        // Verify creator is the original creator
        if listing.creator != creator {
            panic_with_error!(&env, Error::InvalidNftOwner);
        }

        // Check if shares were ever created for this listing
        if listing.total_shares > 0 {
            panic_with_error!(&env, Error::CannotModifyShareStructure);
        }

        // Ensure reserved shares don't exceed total shares
        if reserved_shares > shares_to_add {
            panic_with_error!(&env, Error::InvalidSharesDistribution);
        }

        // Add the shares
        listing.total_shares = shares_to_add;
        listing.available_shares = shares_to_add - reserved_shares;

        // Update ownership shares
        let mut ownership_shares: Map<Address, u32> = Map::new(&env);
        ownership_shares.set(creator.clone(), shares_to_add);

        // Update storage
        store_persistent(&env, &DataKey::Listing(listing_id), &listing);
        store_persistent(
            &env,
            &DataKey::OwnershipShares(listing_id),
            &ownership_shares,
        );

        // Mint NFT shares to the creator
        get_nft_client(&env).mint(&creator, &listing_id, &shares_to_add, &listing.metadata_uri);

        MarketplaceEvent::SharesAdded(listing_id, shares_to_add).publish(&env);
    }

    pub fn update_listing(
        env: Env,
        listing_id: u64,
        reference_id: String,
        new_duration: u64,
        allow_purchase: bool,
        allow_rent: bool,
    ) {
        let mut listing: Listing = get_listing_by_id(&env, listing_id);
        listing.creator.require_auth();
        listing.reference_id = reference_id;
        listing.duration = new_duration;
        listing.allow_purchase = allow_purchase;
        listing.allow_rent = allow_rent;

        store_persistent(&env, &DataKey::Listing(listing_id), &listing);
        MarketplaceEvent::ListingUpdated(listing_id).publish(&env);
    }

    pub fn get_listing(env: Env, listing_id: u64) -> Listing {
        let listing: Listing = get_listing_by_id(&env, listing_id);
        listing
    }

    pub fn get_all_listings(env: Env) -> Vec<Listing> {
        let listing_count: u64 = get_data(&env, &DataKey::ListingCount).unwrap_or(0);
        let mut listings: Vec<Listing> = Vec::<Listing>::new(&env);

        for id in 1..=listing_count {
            if let Some(listing) = get_persistent(&env, &DataKey::Listing(id)) {
                listings.push_back(listing);
            }
        }

        listings
    }

    pub fn get_listing_count(env: Env) -> u64 {
        get_data(&env, &DataKey::ListingCount).unwrap_or(0)
    }

    // Remove
    pub fn update_listing_count(env: Env, count: u64) {
        store_data(&env, &DataKey::ListingCount, &count);
    }

    pub fn remove_listing(env: Env, listing_id: u64) {
        remove_persistent(&env, &DataKey::Listing(listing_id));
    }

    pub fn change_listing_status(env: Env, listing_id: u64, status: ListingStatus) {
        let mut listing: Listing = get_listing_by_id(&env, listing_id);
        listing.status = status;
        store_persistent(&env, &DataKey::Listing(listing_id), &listing);
    }

    pub fn get_listing_current_price(env: Env, listing_id: u64) -> i128 {
        let listing: Listing = get_listing_by_id(&env, listing_id);
        parse_amount(&env, &listing.price)
    }

    pub fn get_usdc_amount(env: Env, amount: i128) -> i128 {
        parse_amount(&env, &amount)
    }

    pub fn get_usdc_price(env: Env) -> (i128, u32) {
        get_usdc_price(&env)
    }

    pub fn rent(env: Env, listing_id: u64, renter: Address, amount: i128, duration: u64) -> u64 {
        renter.require_auth();

        let mut listing: Listing = get_listing_by_id(&env, listing_id);
        if listing.status != ListingStatus::Available {
            panic_with_error!(&env, Error::ListingNotAvailable);
        }

        transfer_and_lock_tokens(
            &env,
            listing_id.clone(),
            amount.clone(),
            &listing.creator,
            &renter,
        );

        let agreement_id = create_purchase_agreement(
            &env,
            &listing_id.into(),
            &renter,
            &listing.creator,
            &duration,
            &0u32,
            &true,
        );

        listing.agreement_id = agreement_id;
        listing.status = ListingStatus::Unavailable;
        store_persistent(&env, &DataKey::Listing(listing_id), &listing);

        MarketplaceEvent::Purchase(
            listing_id,
            agreement_id,
            PurchaseType::Rent,
            listing.creator,
            renter,
        )
        .publish(&env);

        agreement_id
    }

    pub fn purchase(env: Env, listing_id: u64, buyer: Address) -> u64 {
        buyer.require_auth();
        let mut listing = get_listing_by_id(&env, listing_id);

        // Check if the NFT is currently rented
        if listing.status != ListingStatus::Available {
            panic_with_error!(&env, Error::ListingNotAvailable); // Can't transfer while rented
        }

        transfer_and_lock_tokens(
            &env,
            listing_id.clone(),
            listing.price.clone(),
            &listing.creator,
            &buyer,
        );

        // Update ownership shares in marketplace
        let mut ownership_shares: Map<Address, u32> =
            get_persistent(&env, &DataKey::OwnershipShares(listing_id)).unwrap_or(Map::new(&env));

        for (owner, _) in ownership_shares.iter() {
            // Remove all ownership & listings
            ownership_shares.remove(owner.clone());
            remove_listing(&env, listing_id, owner);
        }

        ownership_shares.set(buyer.clone(), listing.total_shares.clone());
        store_persistent(
            &env,
            &DataKey::OwnershipShares(listing_id),
            &ownership_shares,
        );

        // Create a purchase agreement
        let agreement_id: u64 = create_purchase_agreement(
            &env,
            &listing_id.into(),
            &buyer,
            &listing.creator,
            &0u64,
            &listing.total_shares,
            &false,
        );

        listing.agreement_id = agreement_id;
        store_persistent(&env, &DataKey::Listing(listing_id), &listing);

        MarketplaceEvent::Purchase(
            listing_id,
            agreement_id,
            PurchaseType::Buy,
            listing.creator,
            buyer,
        )
        .publish(&env);
        agreement_id
    }

    pub fn purchase_and_confirm(env: Env, listing_id: u64, buyer: Address) -> u64 {
        buyer.require_auth();
        let mut listing = get_listing_by_id(&env, listing_id);

        // Check if the NFT is currently rented
        if listing.status != ListingStatus::Available {
            panic_with_error!(&env, Error::ListingNotAvailable); // Can't transfer while rented
        }

        transfer_tokens_to_owner(&env, listing.price.clone(), &buyer, &listing.creator);

        // Update ownership shares in marketplace
        let mut ownership_shares: Map<Address, u32> =
            get_persistent(&env, &DataKey::OwnershipShares(listing_id)).unwrap_or(Map::new(&env));

        for (owner, _) in ownership_shares.iter() {
            // Remove all ownership & listings
            ownership_shares.remove(owner.clone());
            remove_listing(&env, listing_id, owner);
        }

        ownership_shares.set(buyer.clone(), listing.total_shares.clone());
        store_persistent(
            &env,
            &DataKey::OwnershipShares(listing_id),
            &ownership_shares,
        );

        // Create a purchase agreement
        let agreement_id: u64 = create_purchase_agreement(
            &env,
            &listing_id.into(),
            &buyer,
            &listing.creator,
            &0u64,
            &listing.total_shares,
            &false,
        );

        listing.agreement_id = agreement_id;
        store_persistent(&env, &DataKey::Listing(listing_id), &listing);

        MarketplaceEvent::Purchase(
            listing_id,
            agreement_id,
            PurchaseType::Buy,
            listing.creator,
            buyer,
        )
        .publish(&env);
        agreement_id
    }

    pub fn purchase_shares(
        env: Env,
        buyer: Address,
        seller: Address,
        listing_id: u64,
        shares_to_buy: u32,
    ) -> u64 {
        buyer.require_auth();

        // Get listing
        let mut listing: Listing = get_persistent(&env, &DataKey::Listing(listing_id)).unwrap();

        // Validate purchase
        if listing.total_shares < 1 {
            panic_with_error!(&env, Error::ListingNotAvailable);
        }

        if shares_to_buy == 0 {
            panic_with_error!(&env, Error::InsufficientShares);
        }

        // Get current ownership shares
        let mut ownership_shares: Map<Address, u32> =
            get_persistent(&env, &DataKey::OwnershipShares(listing_id)).unwrap_or(Map::new(&env));

        let seller_shares = ownership_shares.get(seller.clone()).unwrap_or(0);
        let available_shares = if listing.creator == seller {
            listing.available_shares
        } else {
            seller_shares
        };

        if available_shares < shares_to_buy {
            panic_with_error!(&env, Error::InsufficientSharesForPurchase);
        }

        // Calculate price for shares
        let share_price: i128 =
            (listing.price * shares_to_buy as i128) / listing.total_shares as i128;
        // process payment
        transfer_tokens_to_owner(&env, share_price, &buyer, &seller);

        // Update ownership shares
        let buyer_current_shares = ownership_shares.get(buyer.clone()).unwrap_or(0);
        ownership_shares.set(buyer.clone(), buyer_current_shares + shares_to_buy);
        ownership_shares.set(seller.clone(), seller_shares - shares_to_buy);
        store_persistent(
            &env,
            &DataKey::OwnershipShares(listing_id),
            &ownership_shares,
        );

        if seller == listing.creator {
            listing.available_shares -= shares_to_buy;
        }

        // Update listing
        store_persistent(&env, &DataKey::Listing(listing_id), &listing);

        // Mint NFT shares to the buyer
        get_nft_client(&env).transfer_shares(&seller, &buyer.clone(), &listing_id, &shares_to_buy);

        // Create a transfer agreement
        let agreement_id: u64 = create_purchase_agreement(
            &env,
            &listing_id.into(),
            &buyer,
            &seller,
            &0u64,
            &shares_to_buy,
            &false,
        );

        agreement_id
    }

    pub fn confirm_receipt(
        env: Env,
        renter_or_buyer: Address,
        listing_id: u64,
        is_rental: bool,
    ) -> Result<(), Error> {
        renter_or_buyer.require_auth();

        let listing: Listing = get_listing_by_id(&env, listing_id);
        complete_agreement(&env, renter_or_buyer.clone(), listing.clone(), is_rental);

        store_persistent(&env, &DataKey::Listing(listing_id), &listing);
        let payment_amount = get_escrow_client(&env).release(&listing_id);

        distribute_dividends(&env, listing_id, payment_amount)?;
        MarketplaceEvent::ConfirmedReceipt(listing_id, renter_or_buyer).publish(&env);

        Ok(())
    }

    pub fn cancel_sale_or_rental(env: Env, seller: Address, listing_id: u64) -> Result<(), Error> {
        seller.require_auth();
        let mut listing: Listing = get_listing_by_id(&env, listing_id);
        terminate_agreement(&env, &listing.agreement_id, &listing_id, &seller);

        listing.status = ListingStatus::Available;
        store_persistent(&env, &DataKey::Listing(listing_id), &listing);

        MarketplaceEvent::SaleOrRentalCancelled(listing_id, seller).publish(&env);

        Ok(())
    }

    // Owner calls this to confirm renter has returned item and agreement has been reached
    pub fn reclaim_or_return(env: Env, seller: Address, listing_id: u64) -> Result<(), Error> {
        seller.require_auth();
        let mut listing: Listing = get_listing_by_id(&env, listing_id);
        get_agreement_client(&env).complete_agreement(&listing.agreement_id, &seller);
        listing.status = ListingStatus::Available;
        store_persistent(&env, &DataKey::Listing(listing_id), &listing);

        MarketplaceEvent::AssetReclaimed(listing_id, seller).publish(&env);
        Ok(())
    }
}

#[cfg(test)]
mod test;
