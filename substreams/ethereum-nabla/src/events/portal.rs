#![allow(unused_variables)]

use crate::abi;
use crate::abi::nabla_portal::events::{
    AssetRegistered, AssetUnregistered, EthForExactTokensSwapped, ExactTokensForEthSwapped,
    ExactTokensForTokensSwapped, GateUpdated, GatedAccessDisabled, GatedAccessEnabled,
    GuardActivated, GuardDeactivated, GuardOracleSet, OracleAdapterSet, OwnershipTransferred,
    Paused, Unpaused,
};
use crate::modules::initial_state::{router, swap_pool};
use crate::storage::portal::{GATE, GATED, GUARD_ORACLE, ORACLE_ADAPTER, OWNER, PAUSED, ROUTERS};
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

fn get_asset_registered_changed_attributes(storage_changes: &[StorageChange]) -> Vec<Attribute> {
    let mut attributes = Vec::new();

    for change in storage_changes {
        substreams::log::info!("Hex change key: {}", change.key.to_hex());
        if change.key == ROUTERS.slot {
            let old_length = read_bytes(&change.old_value, ROUTERS.offset, ROUTERS.number_of_bytes);
            let new_length = read_bytes(&change.new_value, ROUTERS.offset, ROUTERS.number_of_bytes);
            if old_length != new_length {
                let length = BigInt::from_unsigned_bytes_be(new_length);
                let last_index = &length - BigInt::from(1);
                let element_slot = keccak_hash_slot(&ROUTERS.slot) + last_index;
                for inner_change in storage_changes {
                    if BigInt::from_unsigned_bytes_be(&inner_change.key) == element_slot {
                        let new_element_value = read_bytes(
                            &inner_change.new_value,
                            ROUTERS.offset,
                            ROUTERS.number_of_bytes, // should be 20 bytes
                        );
                        substreams::log::info!(
                            "New router added: {:?}",
                            new_element_value.to_vec().to_hex(),
                        );
                        attributes.push(Attribute {
                            name: "new_router".to_string(),
                            value: new_element_value.into(),
                            change: ChangeType::Update.into(),
                        });
                    }
                }
            }
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
            attributes: get_asset_registered_changed_attributes(storage_changes),
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

fn extract_attribute_if_changed(
    change: &StorageChange,
    loc: &StorageLocation,
) -> Option<Attribute> {
    let old_value = read_bytes(&change.old_value, loc.offset, loc.number_of_bytes);
    let new_value = read_bytes(&change.new_value, loc.offset, loc.number_of_bytes);
    (old_value != new_value).then(|| Attribute {
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
                .find(|change| change.key == loc.slot)
                .and_then(|change| extract_attribute_if_changed(change, loc))
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
