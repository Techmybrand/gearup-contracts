use soroban_sdk::{contract, contractimpl, vec, Address, BytesN, Env, Map, String, Symbol, Vec};

use crate::{
    events::NFTEvent,
    storage::{
        get_data, get_persistent, has_data, has_persistent, remove_data, remove_persistent,
        store_data, store_persistent,
    },
    utils::require_marketplace_call,
};
use common::nft::{
    interface::NFTInterface,
    types::{DataKey, Error, TokenMetadata, ADMIN, MARKETPLACE_CONTRACT},
};

const NAME: &str = "GearUp Tokenized Asset";
const SYMBOL: &str = "GUTA";

#[contract]
pub struct NFTContract;

#[contractimpl]
impl NFTInterface for NFTContract {
    fn initialize(env: Env, admin: Address, marketplace_contract_id: Address) -> Result<(), Error> {
        admin.require_auth();
        if env.storage().instance().has::<Symbol>(&ADMIN) {
            return Err(Error::AlreadyInitialized);
        }
        store_data(&env, &ADMIN, &admin);
        store_data(&env, &MARKETPLACE_CONTRACT, &marketplace_contract_id);
        NFTEvent::Initialized.publish(&env);
        Ok(())
    }

    fn version() -> u32 {
        1
    }

    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        let admin: Address = get_data(&env, &ADMIN).unwrap();
        admin.require_auth();
        env.deployer().update_current_contract_wasm(new_wasm_hash);
        NFTEvent::Upgraded(Self::version()).publish(&env);
    }

    fn update_state(env: Env, state_key: Symbol, state_value: Address) -> Result<(), Error> {
        let admin: Address = get_data(&env, &ADMIN).unwrap();
        admin.require_auth();

        if !has_data::<Symbol>(&env, &state_key) {
            return Err(Error::StateNotAlreadySet);
        }

        store_data(&env, &state_key, &state_value);
        env.events()
            .publish(("state_updated", state_key), state_value);

        Ok(())
    }

    fn name(env: Env) -> String {
        String::from_str(&env, NAME)
    }

    fn symbol(env: Env) -> String {
        String::from_str(&env, SYMBOL)
    }

    fn mint(env: Env, to: Address, token_id: u64, shares: u32, token_uri: String) -> u64 {
        require_marketplace_call(&env);

        if !has_data(&env, &DataKey::TokenMetadata(token_id)) {
            let metadata: TokenMetadata =
                match get_persistent(&env, &DataKey::TokenMetadata(token_id)) {
                    Some(meta) => meta,
                    None => TokenMetadata {
                        total_shares: shares,
                        token_uri,
                    },
                };
            store_persistent(&env, &DataKey::TokenMetadata(token_id), &metadata);
        }

        let mut ownership: Map<Address, u32> =
            get_persistent(&env, &DataKey::TokenOwnership(token_id))
                .unwrap_or_else(|| Map::new(&env));
        let current_shares: u32 = ownership.get(to.clone()).unwrap_or(0);
        ownership.set(to.clone(), current_shares + shares);

        store_persistent(&env, &DataKey::TokenOwnership(token_id), &ownership);

        NFTEvent::Mint(token_id, to).publish(&env);

        token_id
    }

    fn transfer(env: Env, from: Address, to: Address, token_id: u64) -> bool {
        require_marketplace_call(&env);

        let mut ownership: Map<Address, u32> =
            get_persistent(&env, &DataKey::TokenOwnership(token_id)).unwrap();

        let metadata: TokenMetadata =
            get_persistent(&env, &DataKey::TokenMetadata(token_id)).unwrap();

        let owners: Map<Address, u32> = ownership.clone();
        for (owner, _) in owners.iter() {
            // Remove sender's ownership
            ownership.remove(owner);
        }

        // Set recipient as sole owner
        ownership.set(to.clone(), metadata.total_shares);
        store_persistent(&env, &DataKey::TokenOwnership(token_id), &ownership);

        // If there's a temporary control, revoke it
        if has_data(&env, &DataKey::TemporaryControl(token_id)) {
            remove_data(&env, &DataKey::TemporaryControl(token_id));
        }

        NFTEvent::Transfer(token_id, from, to).publish(&env);
        true
    }

    // Transfer shares from one owner to another
    fn transfer_shares(env: Env, from: Address, to: Address, token_id: u64, shares: u32) -> bool {
        require_marketplace_call(&env);

        let mut ownership: Map<Address, u32> =
            get_persistent(&env, &DataKey::TokenOwnership(token_id)).unwrap();

        // Check if sender has enough shares
        let from_shares: u32 = ownership.get(from.clone()).unwrap_or(0);
        if from_shares < shares {
            return false;
        }

        // Update sender's shares
        if from_shares == shares {
            ownership.remove(from.clone());
        } else {
            ownership.set(from.clone(), from_shares - shares);
        }

        // Update recipient's shares
        let to_shares = ownership.get(to.clone()).unwrap_or(0);
        ownership.set(to.clone(), to_shares + shares);

        store_persistent(&env, &DataKey::TokenOwnership(token_id), &ownership);
        NFTEvent::TransferShares(token_id, from, to, shares).publish(&env);
        true
    }

    // Burn shares of a token (remove from circulation)
    fn burn_shares(env: Env, owner: Address, token_id: u64, shares: u32) -> bool {
        owner.require_auth();

        // Get ownership map
        let mut ownership: Map<Address, u32> =
            get_persistent(&env, &DataKey::TokenOwnership(token_id)).unwrap();

        // Check if sender has enough shares
        let owner_shares: u32 = ownership.get(owner.clone()).unwrap_or(0);
        if owner_shares < shares {
            return false;
        }

        // Update owner's shares
        if owner_shares == shares {
            ownership.remove(owner.clone());
        } else {
            ownership.set(owner.clone(), owner_shares - shares);
        }

        store_persistent(&env, &DataKey::TokenOwnership(token_id), &ownership);

        // Get metadata to update total shares
        if let Some(mut metadata) =
            get_persistent::<DataKey, TokenMetadata>(&env, &DataKey::TokenMetadata(token_id))
        {
            metadata.total_shares -= shares;

            // If no shares left, remove the token completely
            if metadata.total_shares == 0 {
                remove_persistent(&env, &DataKey::TokenMetadata(token_id));
                remove_persistent(&env, &DataKey::TokenOwnership(token_id));
                if has_data(&env, &DataKey::TemporaryControl(token_id)) {
                    remove_data(&env, &DataKey::TemporaryControl(token_id));
                }
            } else {
                // Otherwise update metadata with new total
                store_persistent(&env, &DataKey::TokenMetadata(token_id), &metadata);
            }

            true
        } else {
            false
        }
    }

    // Check if an address is the sole owner of the NFT
    fn is_sole_owner(env: Env, token_id: u64, address: Address) -> bool {
        let ownership: Option<Map<Address, u32>> =
            get_persistent(&env, &DataKey::TokenOwnership(token_id));
        let metadata: Option<TokenMetadata> =
            get_persistent(&env, &DataKey::TokenMetadata(token_id));

        if let (Some(owners), Some(meta)) = (ownership, metadata) {
            if let Some(shares) = owners.get(address) {
                return shares == meta.total_shares;
            }
        }

        false
    }

    // Merge shares owned by the same address (bookkeeping function)
    fn merge_shares(env: Env, owner: Address, token_id: u64) -> u32 {
        owner.require_auth();
        let ownership: Map<Address, u32> =
            get_persistent(&env, &DataKey::TokenOwnership(token_id)).unwrap();
        ownership.get(owner).unwrap_or(0)
    }

    // Grant temporary control for rentals
    fn grant_temporary_control(env: Env, token_id: u64, renter: Address, end_time: u64) {
        require_marketplace_call(&env);
        store_data(
            &env,
            &DataKey::TemporaryControl(token_id),
            &(renter, end_time),
        );
    }

    // Revoke temporary control
    fn revoke_temporary_control(env: Env, token_id: u64, renter: Address) {
        require_marketplace_call(&env);
        let control: (Address, u64) = get_data(&env, &DataKey::TemporaryControl(token_id)).unwrap();
        if control.0 == renter {
            remove_data(&env, &DataKey::TemporaryControl(token_id));
        }
    }

    // Check if an address has control over a token (ownership or temporary rental)
    fn has_control(env: Env, token_id: u64, address: Address) -> bool {
        // Check if address has ownership
        let ownership: Option<Map<Address, u32>> =
            get_persistent(&env, &DataKey::TokenOwnership(token_id));
        if let Some(owners) = ownership {
            if owners.get(address.clone()).unwrap_or(0) > 0 {
                return true;
            }
        }

        // Check if address has temporary control
        let temp_control: Option<(Address, u64)> =
            get_data(&env, &DataKey::TemporaryControl(token_id));
        if let Some((controller, end_time)) = temp_control {
            if controller == address && env.ledger().timestamp() < end_time {
                return true;
            }
        }

        false
    }

    // Get token balance (shares) for an address
    fn balance_of(env: Env, token_id: u64, owner: Address) -> u32 {
        let ownership: Option<Map<Address, u32>> =
            get_persistent(&env, &DataKey::TokenOwnership(token_id));

        if let Some(owners) = ownership {
            owners.get(owner).unwrap_or(0)
        } else {
            0
        }
    }

    // Get total supply (total shares) for a token
    fn total_supply(env: Env, token_id: u64) -> u32 {
        if let Some(metadata) =
            get_persistent::<DataKey, TokenMetadata>(&env, &DataKey::TokenMetadata(token_id))
        {
            metadata.total_shares
        } else {
            0
        }
    }

    // Get all owners and their shares for a token
    fn get_all_owners(env: Env, token_id: u64) -> Option<Map<Address, u32>> {
        get_persistent(&env, &DataKey::TokenOwnership(token_id))
    }

    fn token_uri(env: Env, token_id: u64) -> String {
        let metadata: TokenMetadata =
            get_persistent(&env, &DataKey::TokenMetadata(token_id)).unwrap();
        metadata.token_uri
    }

    fn set_token_uri(env: Env, token_id: u64, uri: String) {
        // Only admin or marketplace can call this
        require_marketplace_call(&env);

        if let Some(mut metadata) = env
            .storage()
            .instance()
            .get::<DataKey, TokenMetadata>(&DataKey::TokenMetadata(token_id))
        {
            metadata.token_uri = uri;
            // Save updated metadata
            store_persistent(&env, &DataKey::TokenMetadata(token_id), &metadata);
        }
    }

    fn exists(env: Env, token_id: u64) -> bool {
        has_persistent(&env, &DataKey::TokenMetadata(token_id))
    }

    fn get_metadata(env: Env, token_id: u64) -> Option<TokenMetadata> {
        env.storage()
            .instance()
            .get(&DataKey::TokenMetadata(token_id))
    }

    fn owners_of(env: Env, token_id: u64) -> Vec<Address> {
        let tokens =
            get_persistent::<DataKey, Map<Address, u32>>(&env, &DataKey::TokenOwnership(token_id))
                .unwrap();
        tokens.keys()
    }

    fn tokens_of_owner(env: Env, owner: Address) -> Vec<(u64, u32)> {
        let mut result = Vec::new(&env);

        // Get current listing count from marketplace to know max token ID
        let marketplace: Address = get_data(&env, &MARKETPLACE_CONTRACT).unwrap();
        let args = vec![&env];
        let listing_count: u64 =
            env.invoke_contract(&marketplace, &Symbol::new(&env, "get_listing_count"), args);

        // Check each token ID up to listing count
        for token_id in 1..=listing_count {
            if let Some(ownership) = get_persistent::<DataKey, Map<Address, u32>>(
                &env,
                &DataKey::TokenOwnership(token_id),
            ) {
                if let Some(shares) = ownership.get(owner.clone()) {
                    result.push_back((token_id, shares));
                }
            }
        }

        result
    }
}
