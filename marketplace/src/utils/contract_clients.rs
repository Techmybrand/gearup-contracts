use crate::{
    oracle::oracle, storage::get_data, types::{AGREEMENT_CONTRACT, ESCROW_CONTRACT, NFT_CONTRACT, PRICE_FEED_CONTRACT, REFLECTOR_ORACLE}
};
use common::{
    agreement::interface::AgreementContractClient, escrow::interface::EscrowContractClient,
    nft::interface::NFTContractClient, pricefeed::interface::PriceOracleContractClient,
};
use soroban_sdk::{Address, Env};

pub fn get_nft_client(env: &Env) -> NFTContractClient<'_> {
    let nft_ca: Address = get_data(env, &NFT_CONTRACT).unwrap();
    NFTContractClient::new(&env, &nft_ca)
}

pub fn get_agreement_client(env: &Env) -> AgreementContractClient<'_> {
    let agreement_ca: Address = get_data(env, &AGREEMENT_CONTRACT).unwrap();
    AgreementContractClient::new(&env, &agreement_ca)
}

pub fn get_escrow_client(env: &Env) -> EscrowContractClient<'_> {
    let escrow_ca: Address = get_data(env, &ESCROW_CONTRACT).unwrap();
    EscrowContractClient::new(&env, &escrow_ca)
}

pub fn get_feed_client(env: &Env) -> PriceOracleContractClient<'_> {
    let price_feed_ca: Address = get_data(env, &PRICE_FEED_CONTRACT).unwrap();
    PriceOracleContractClient::new(&env, &price_feed_ca)
}

pub fn get_oracle_client(env: &Env) -> oracle::Client {
    let reflector_ca: Address = get_data(env, &REFLECTOR_ORACLE).unwrap();
    oracle::Client::new(env, &reflector_ca)
}
