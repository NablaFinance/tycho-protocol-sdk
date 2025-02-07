#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use crate::abi;
use crate::abi::nabla_portal::events::{
    AssetRegistered, AssetUnregistered, EthForExactTokensSwapped, ExactTokensForEthSwapped,
    ExactTokensForTokensSwapped, GateUpdated, GatedAccessDisabled, GatedAccessEnabled,
    GuardActivated, GuardDeactivated, GuardOracleSet, OracleAdapterSet, OwnershipTransferred,
    Paused, Unpaused,
};
use crate::modules::initial_state::{router, swap_pool};
use crate::storage::portal::{
    ASSETS_BY_ROUTER, GATE, GATED, GUARD_ON, GUARD_ORACLE, ORACLE_ADAPTER, OWNER, PAUSED, ROUTERS,
    ROUTER_ASSETS,
};
use crate::storage::utils::StorageType;
use crate::storage::utils::{read_bytes, StorageLocation};
use substreams::scalar::BigInt;
use substreams_ethereum::pb::eth::v2::{Log, StorageChange};
use substreams_ethereum::Event;
use substreams_helper::hex::Hexable;
use tiny_keccak::{Hasher, Keccak};
use tycho_substreams::prelude::{Attribute, ChangeType, EntityChanges};

pub trait EventTrait {
    fn get_entity_changes(
        &self,
        log: &Log,
        storage_changes: &[StorageChange],
    ) -> Vec<EntityChanges>;
}

pub enum EventType {
    AssetRegistered(AssetRegistered),
    AssetUnregistered(AssetUnregistered),
    EthForExactTokensSwapped(EthForExactTokensSwapped),
    ExactTokensForEthSwapped(ExactTokensForEthSwapped),
    ExactTokensForTokensSwapped(ExactTokensForTokensSwapped),
    GuardActivated(GuardActivated),
    GuardOracleSet(GuardOracleSet),
    GuardDeactivated(GuardDeactivated),
    OracleAdapterSet(OracleAdapterSet),
    Paused(Paused),
    Unpaused(Unpaused),
    GatedAccessEnabled(GatedAccessEnabled),
    GatedAccessDisabled(GatedAccessDisabled),
    OwnershipTransferred(OwnershipTransferred),
    GateUpdated(GateUpdated),
}

impl EventType {
    pub fn as_event_trait(&self) -> &dyn EventTrait {
        match self {
            EventType::AssetRegistered(e) => e,
            EventType::AssetUnregistered(e) => e,
            EventType::EthForExactTokensSwapped(e) => e,
            EventType::ExactTokensForEthSwapped(e) => e,
            EventType::ExactTokensForTokensSwapped(e) => e,
            EventType::GuardActivated(e) => e,
            EventType::GuardOracleSet(e) => e,
            EventType::GuardDeactivated(e) => e,
            EventType::OracleAdapterSet(e) => e,
            EventType::Paused(e) => e,
            EventType::Unpaused(e) => e,
            EventType::GatedAccessEnabled(e) => e,
            EventType::GatedAccessDisabled(e) => e,
            EventType::OwnershipTransferred(e) => e,
            EventType::GateUpdated(e) => e,
        }
    }
}

fn default() -> Vec<EntityChanges> {
    vec![EntityChanges { component_id: "default".into(), attributes: vec![] }]
}

fn keccak_hash_slot(slot: &[u8]) -> BigInt {
    let mut hasher = Keccak::v256();
    let mut hashed_slot = [0u8; 32];
    hasher.update(slot);
    hasher.finalize(&mut hashed_slot);
    BigInt::from_unsigned_bytes_be(&hashed_slot)
}

fn compute_element_slot(slot: &[u8], new_length: &[u8]) -> BigInt {
    keccak_hash_slot(slot) + BigInt::from_unsigned_bytes_be(new_length) - BigInt::from(1)
}

fn read_item_at_slot(
    element_slot: BigInt,
    storage_changes: &[StorageChange],
    storage_type: &StorageType,
) -> Result<Vec<u8>, String> {
    storage_changes
        .iter()
        .find(|change| BigInt::from_unsigned_bytes_be(&change.key) == element_slot)
        .map(|inner_change| {
            let number_of_bytes = storage_type
                .item_type()?
                .number_of_bytes();
            Ok(read_bytes(&inner_change.new_value, 0, number_of_bytes).to_vec())
        })
        .transpose()
        .and_then(|opt| {
            opt.ok_or_else(|| format!("Failed to find new element for slot: {}", element_slot))
        })
}

fn keccak256(input: &[u8]) -> [u8; 32] {
    use tiny_keccak::Keccak;
    let mut hasher = Keccak::v256();
    let mut output = [0u8; 32];
    hasher.update(input);
    hasher.finalize(&mut output);
    output
}

fn compute_mapping_key<T: AsRef<[u8]>>(key: T, slot: &[u8; 32]) -> [u8; 32] {
    let mut input = Vec::new();
    input.extend_from_slice(&key.as_ref());
    input.extend_from_slice(&slot.as_ref());
    keccak256(&input)
}

fn pad_address(addr: &[u8]) -> [u8; 32] {
    let mut padded = [0u8; 32];
    padded[12..].copy_from_slice(addr);
    padded
}

fn get_asset_registered_changed_attributes(
    storage_changes: &[StorageChange],
    log: &Log,
) -> Vec<Attribute> {
    let mut attributes = Vec::new();

    let event = abi::nabla_portal::events::AssetRegistered::match_and_decode(log).unwrap();
    let router = pad_address(&event.router);
    let asset = pad_address(&event.asset);

    let assets_by_router_key = compute_mapping_key(&router, &ASSETS_BY_ROUTER.slot);
    let nested_key = compute_mapping_key(&asset, &assets_by_router_key);
    let router_assets_key = compute_mapping_key(&router, &ROUTER_ASSETS.slot);

    for change in storage_changes {
        if change.key == assets_by_router_key {
            substreams::log::info!("Key belongs to ASSETS_BY_ROUTER mapping");
        } else if change.key == nested_key {
            substreams::log::info!("Key belongs to NESTED ASSETS_BY_ROUTER mapping");
        } else if change.key == router_assets_key {
            substreams::log::info!("Key belongs to ROUTER_ASSETS mapping");
        }

        // substreams::log::info!(
        //     "Portal Storage Change Key: {} - {} - {} - {}",
        //     change.key.to_hex(),
        //     assets_by_router_key.to_vec().to_hex(),
        //     nested_key.to_vec().to_hex(),
        //     router_assets_key.to_vec().to_hex(),
        // );

        if let Some(new_length) = new_value_if_changed(change, &ROUTERS) {
            let element_slot = compute_element_slot(&ROUTERS.slot, new_length);
            match read_item_at_slot(element_slot, storage_changes, &ROUTERS.storage_type) {
                Ok(new_element_value) => {
                    substreams::log::info!("New router added: {:?}", new_element_value.to_hex());
                    attributes.push(Attribute {
                        name: "new_router".to_string(),
                        value: new_element_value.into(),
                        change: ChangeType::Update.into(),
                    });
                }
                Err(e) => {
                    substreams::log::info!("Failed to read new router: {}", e);
                }
            }
        }

        if let Some(new_value) = new_value_if_changed(change, &ROUTER_ASSETS) {
            substreams::log::info!("Router Assets changed");
            // let key = new_value; // You may need to cast or parse the key depending on its type
            // let element_slot = compute_element_slot(&ROUTER_ASSETS.slot, &key);
            // match read_item_at_slot(element_slot, storage_changes, &ROUTER_ASSETS.storage_type) {
            //     Ok(updated_value) => {
            //         substreams::log::info!("Router assets updated for key: {:?}", key);
            //         attributes.push(Attribute {
            //             name: "updated_router_asset".to_string(),
            //             value: updated_value.into(),
            //             change: ChangeType::Update.into(),
            //         });
            //     }
            //     Err(e) => {
            //         substreams::log::info!(
            //             "Failed to read updated router asset for key {:?}: {}",
            //             key,
            //             e
            //         );
            //     }
            // }
        }
    }
    attributes
}

impl EventTrait for AssetRegistered {
    fn get_entity_changes(
        &self,
        log: &Log,
        storage_changes: &[StorageChange],
    ) -> Vec<EntityChanges> {
        substreams::log::info!("Getting AssetRegistered Entity Changes");

        let portal_entity_changes = EntityChanges {
            component_id: self.sender.to_hex(),
            attributes: get_asset_registered_changed_attributes(storage_changes, log),
        };

        let router_id: String = self.router.to_hex();
        let asset_id = self.asset.to_hex();
        let pool_address = abi::nabla_router::functions::PoolByAsset { param0: self.asset.clone() }
            .call(self.router.clone())
            .ok_or_else(|| {
                format!(
                    "Failed to retrieve pool_address for asset {} from the router {}",
                    asset_id, router_id
                )
            })
            .expect("TODO: propagate error");

        let router_entity_changes = router::read_state(self.router.clone()).unwrap(); // TODO
        let swap_pool_entity_changes = swap_pool::read_state(pool_address).unwrap(); // TODO
        vec![portal_entity_changes, router_entity_changes, swap_pool_entity_changes]
    }
}

impl EventTrait for AssetUnregistered {
    fn get_entity_changes(
        &self,
        log: &Log,
        storage_changes: &[StorageChange],
    ) -> Vec<EntityChanges> {
        default()
    }
}

impl EventTrait for EthForExactTokensSwapped {
    fn get_entity_changes(
        &self,
        log: &Log,
        storage_changes: &[StorageChange],
    ) -> Vec<EntityChanges> {
        default()
    }
}

impl EventTrait for ExactTokensForEthSwapped {
    fn get_entity_changes(
        &self,
        log: &Log,
        storage_changes: &[StorageChange],
    ) -> Vec<EntityChanges> {
        default()
    }
}

impl EventTrait for ExactTokensForTokensSwapped {
    fn get_entity_changes(
        &self,
        log: &Log,
        storage_changes: &[StorageChange],
    ) -> Vec<EntityChanges> {
        default()
    }
}

impl EventTrait for GuardActivated {
    fn get_entity_changes(
        &self,
        log: &Log,
        storage_changes: &[StorageChange],
    ) -> Vec<EntityChanges> {
        default()
    }
}

impl EventTrait for GuardDeactivated {
    fn get_entity_changes(
        &self,
        log: &Log,
        storage_changes: &[StorageChange],
    ) -> Vec<EntityChanges> {
        default()
    }
}

impl EventTrait for GuardOracleSet {
    fn get_entity_changes(
        &self,
        log: &Log,
        storage_changes: &[StorageChange],
    ) -> Vec<EntityChanges> {
        vec![slot_entity_changes(&log.address, storage_changes, &[GUARD_ORACLE])]
    }
}

impl EventTrait for OracleAdapterSet {
    fn get_entity_changes(
        &self,
        log: &Log,
        storage_changes: &[StorageChange],
    ) -> Vec<EntityChanges> {
        vec![slot_entity_changes(&log.address, storage_changes, &[ORACLE_ADAPTER])]
    }
}

impl EventTrait for Paused {
    fn get_entity_changes(
        &self,
        log: &Log,
        storage_changes: &[StorageChange],
    ) -> Vec<EntityChanges> {
        vec![slot_entity_changes(&log.address, storage_changes, &[PAUSED])]
    }
}

impl EventTrait for Unpaused {
    fn get_entity_changes(
        &self,
        log: &Log,
        storage_changes: &[StorageChange],
    ) -> Vec<EntityChanges> {
        vec![slot_entity_changes(&log.address, storage_changes, &[PAUSED])]
    }
}

impl EventTrait for GateUpdated {
    fn get_entity_changes(
        &self,
        log: &Log,
        storage_changes: &[StorageChange],
    ) -> Vec<EntityChanges> {
        vec![slot_entity_changes(&log.address, storage_changes, &[GATE])]
    }
}

impl EventTrait for GatedAccessEnabled {
    fn get_entity_changes(
        &self,
        log: &Log,
        storage_changes: &[StorageChange],
    ) -> Vec<EntityChanges> {
        vec![slot_entity_changes(&log.address, storage_changes, &[GATED])]
    }
}

impl EventTrait for GatedAccessDisabled {
    fn get_entity_changes(
        &self,
        log: &Log,
        storage_changes: &[StorageChange],
    ) -> Vec<EntityChanges> {
        vec![slot_entity_changes(&log.address, storage_changes, &[GATED])]
    }
}

impl EventTrait for OwnershipTransferred {
    fn get_entity_changes(
        &self,
        log: &Log,
        storage_changes: &[StorageChange],
    ) -> Vec<EntityChanges> {
        vec![slot_entity_changes(&log.address, storage_changes, &[OWNER])]
    }
}

fn new_value_if_changed<'a>(change: &'a StorageChange, loc: &StorageLocation) -> Option<&'a [u8]> {
    (change.key == loc.slot)
        .then_some((
            read_bytes(&change.old_value, loc.offset, loc.storage_type.number_of_bytes()),
            read_bytes(&change.new_value, loc.offset, loc.storage_type.number_of_bytes()),
        ))
        .filter(|(old, new)| old != new)
        .map(|(_, new)| new)
}

fn extract_attribute_if_changed(
    change: &StorageChange,
    loc: &StorageLocation,
) -> Option<Attribute> {
    new_value_if_changed(change, loc).map(|new_value| Attribute {
        name: loc.name.to_string(),
        value: new_value.into(),
        change: ChangeType::Update.into(),
    })
}

fn extract_storage_attributes(
    storage_changes: &[StorageChange],
    locations: &[StorageLocation],
) -> Vec<Attribute> {
    locations
        .iter()
        .flat_map(|loc| {
            storage_changes
                .iter()
                .filter_map(|change| extract_attribute_if_changed(change, loc))
        })
        .collect()
}

fn slot_entity_changes(
    address: &Vec<u8>,
    storage_changes: &[StorageChange],
    locations: &[StorageLocation],
) -> EntityChanges {
    EntityChanges {
        component_id: address.to_hex(),
        attributes: extract_storage_attributes(storage_changes, locations),
    }
}

pub fn decode_event(event: &Log) -> Option<EventType> {
    [
        AssetRegistered::match_and_decode(event).map(EventType::AssetRegistered),
        AssetUnregistered::match_and_decode(event).map(EventType::AssetUnregistered),
        EthForExactTokensSwapped::match_and_decode(event).map(EventType::EthForExactTokensSwapped),
        ExactTokensForEthSwapped::match_and_decode(event).map(EventType::ExactTokensForEthSwapped),
        ExactTokensForTokensSwapped::match_and_decode(event)
            .map(EventType::ExactTokensForTokensSwapped),
        GuardActivated::match_and_decode(event).map(EventType::GuardActivated),
        GuardOracleSet::match_and_decode(event).map(EventType::GuardOracleSet),
        GuardDeactivated::match_and_decode(event).map(EventType::GuardDeactivated),
        OracleAdapterSet::match_and_decode(event).map(EventType::OracleAdapterSet),
        Paused::match_and_decode(event).map(EventType::Paused),
        Unpaused::match_and_decode(event).map(EventType::Unpaused),
        GatedAccessEnabled::match_and_decode(event).map(EventType::GatedAccessEnabled),
        GatedAccessDisabled::match_and_decode(event).map(EventType::GatedAccessDisabled),
        OwnershipTransferred::match_and_decode(event).map(EventType::OwnershipTransferred),
        GateUpdated::match_and_decode(event).map(EventType::GateUpdated),
    ]
    .into_iter()
    .find_map(std::convert::identity)
}
