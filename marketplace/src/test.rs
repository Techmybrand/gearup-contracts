#![cfg(test)]
extern crate std;

use super::*;
use agreement::AgreementContract;
use common::agreement::interface::AgreementContractClient;
use common::escrow::interface::EscrowContractClient;
use common::nft::interface::NFTContractClient;
use common::pricefeed::interface::PriceOracleContractClient;
use escrow::EscrowContract;
use nft::contract::NFTContract;
use price_feed::PriceOracleContract;
use soroban_sdk::testutils::{Address as _, StellarAssetContract};
use soroban_sdk::{token, Address};

fn create_marketplace_contract<'a>(env: &Env) -> MarketplaceContractClient<'a> {
    let contract_id = env.register(MarketplaceContract, ());
    let contract_client = MarketplaceContractClient::new(&env, &contract_id);
    contract_client
}

fn create_nft_contract<'a>(env: &Env) -> NFTContractClient<'a> {
    let contract_id: Address = env.register(NFTContract, ());
    let contract_client: NFTContractClient<'a> = NFTContractClient::new(&env, &contract_id);
    contract_client
}

fn create_agreement_contract<'a>(env: &Env) -> AgreementContractClient<'a> {
    let contract_id: Address = env.register(AgreementContract, ());
    let contract_client: AgreementContractClient<'a> =
        AgreementContractClient::new(env, &contract_id);
    contract_client
}

fn create_escrow_contract<'a>(env: &Env) -> EscrowContractClient<'a> {
    let contract_id: Address = env.register(EscrowContract, ());
    let contract_client: EscrowContractClient<'a> = EscrowContractClient::new(env, &contract_id);
    contract_client
}

fn create_price_feed_contract<'a>(env: &Env) -> PriceOracleContractClient<'a> {
    let contract_id: Address = env.register(PriceOracleContract, ());
    let contract_client: PriceOracleContractClient<'_> =
        PriceOracleContractClient::new(env, &contract_id);
    contract_client
}

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let sac: StellarAssetContract = e.register_stellar_asset_contract_v2(admin.clone());
    (
        token::Client::new(e, &sac.address()),
        token::StellarAssetClient::new(e, &sac.address()),
    )
}

pub struct MarketplaceTest {
    env: Env,
    marketplace_client: MarketplaceContractClient<'static>,
    nft_client: NFTContractClient<'static>,
    agreement_client: AgreementContractClient<'static>,
    escrow_client: EscrowContractClient<'static>,
    token_client: token::TokenClient<'static>,
    price_feed_client: PriceOracleContractClient<'static>,
    alice: Address,
    bob: Address,
    admin: Address,
}

impl MarketplaceTest {
    fn setup() -> Self {
        let env: Env = Env::default();
        let test = Self::setup_no_init(env.clone());
        let reflector_ca: Address = Address::generate(&env);
        let payment_token: Address = Address::generate(&env);

        let initial_rate: i128 = 1612_0000000;

        test.marketplace_client.initialize(
            &test.admin,
            &test.nft_client.address,
            &test.agreement_client.address,
            &test.escrow_client.address,
            &test.price_feed_client.address,
            &reflector_ca,
            &payment_token
        );
        test.nft_client
            .initialize(&test.admin, &test.marketplace_client.address);
        test.agreement_client
            .initialize(&test.admin, &test.marketplace_client.address);
        test.escrow_client
            .initialize(&test.admin, &test.marketplace_client.address);
        test.price_feed_client.initialize(
            &test.admin,
            &initial_rate,
            &3600_u64,
            &1u64,
            &10000_i128,
        );

        return test;
    }

    fn setup_no_init(env: Env) -> Self {
        env.mock_all_auths();

        let marketplace_client: MarketplaceContractClient<'_> = create_marketplace_contract(&env);
        let agreement_client: AgreementContractClient<'_> = create_agreement_contract(&env);
        let escrow_client: EscrowContractClient<'_> = create_escrow_contract(&env);
        let nft_client: NFTContractClient<'_> = create_nft_contract(&env);
        let price_feed_client: PriceOracleContractClient<'_> = create_price_feed_contract(&env);

        // Generate the accounts (users)
        let alice: Address = Address::generate(&env);
        let bob: Address = Address::generate(&env);
        let admin: Address = Address::generate(&env);

        assert_ne!(alice, bob);
        assert_ne!(alice, admin);
        assert_ne!(bob, admin);

        let (token_client, token_admin_client) = create_token_contract(&env, &admin);
        token_admin_client.mint(&bob, &10_000_0000000_i128);

        return MarketplaceTest {
            env,
            marketplace_client,
            nft_client,
            agreement_client,
            escrow_client,
            token_client,
            price_feed_client,
            alice,
            bob,
            admin,
        };
    }
}

mod create_listing;
mod purchase_or_rent;
