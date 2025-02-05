#![allow(unused_variables)]

use crate::events::portal;
use crate::modules::map_components::ComponentType;
use crate::pb::nabla::v1::{component::Kind, Component};
use anyhow::Result;
use itertools::Itertools;

use substreams::store::{StoreGet, StoreGetProto};
use substreams_ethereum::pb::eth;
use substreams_ethereum::pb::eth::v2::{Log, StorageChange};
use substreams_helper::storage_change::StorageChangesFilter;

use substreams_helper::hex::Hexable;
use tycho_substreams::prelude::{BlockChanges, EntityChanges};

impl From<Kind> for ComponentType {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Portal(_) => ComponentType::Portal,
            Kind::Router(_) => ComponentType::Router,
            Kind::SwapPool(_) => ComponentType::SwapPool,
        }
    }
}

fn default(
    log: &Log,
    storage_changes: &[StorageChange],
) -> Option<Result<Vec<EntityChanges>, String>> {
    let component_id = log.address.to_hex();
    Some(Ok(vec![EntityChanges { component_id, attributes: vec![] }]))
}

fn portal_event(
    log: &Log,
    storage_changes: &[StorageChange],
) -> Option<Result<Vec<EntityChanges>, String>> {
    substreams::log::info!("Getting Portal Event Changes");
    let entity_changes = portal::decode_event(log).map(|e| {
        e.as_event_trait()
            .get_entity_changes(storage_changes)
    })?;
    substreams::log::info!("Portal Entity Changes: {:?}", entity_changes);
    let component_id = log.address.to_hex();
    Some(Ok(vec![EntityChanges { component_id, attributes: vec![] }]))
}
// fn router_event(log: &Log) -> Result<(), String> {}
// fn swap_pool_event(log: &Log) -> Result<(), String> {}

#[substreams::handlers::map]
pub fn map_block_changes(
    block: eth::v2::Block,
    component_store: StoreGetProto<Component>,
) -> Result<BlockChanges, String> {
    let entity_changes: Vec<EntityChanges> = block
        .transactions()
        .flat_map(|tx| {
            tx.logs_with_calls()
                .filter_map(|(log, call_view)| {
                    if let Some(component) = component_store.get_last(&log.address.to_hex()) {
                        substreams::log::info!("Stored component found: {:?}", component);
                        let storage_changes: Vec<StorageChange> = call_view
                            .call
                            .storage_changes
                            .filter_by_address(
                                log.address
                                    .as_slice()
                                    .try_into()
                                    .expect("Address is not 20 bytes long"),
                            )
                            .into_iter()
                            .cloned()
                            .collect();
                        substreams::log::info!("Storage changes found: {:?}", storage_changes);
                        match component
                            .kind
                            .expect("Kind not set")
                            .into()
                        {
                            ComponentType::Portal => portal_event(log, &storage_changes),
                            ComponentType::Router => default(log, &storage_changes),
                            ComponentType::SwapPool => default(log, &storage_changes),
                        }
                    } else {
                        None
                    }
                })
        })
        .flatten_ok()
        .try_collect()?;

    // let changes = block_tx_protocol_components
    //     .tx_components
    //     .iter()
    //     .map(|tx_components| {
    //         let entity_changes = tx_components
    //             .components
    //             .iter()
    //             .map(initialize_component_state)
    //             .collect::<Result<Vec<EntityChanges>, _>>()
    //             .expect("Failed to collect entity changes");

    //         TransactionEntityChanges {
    //             tx: tx_components.tx.clone(),
    //             entity_changes,
    //             component_changes: vec![],
    //             balance_changes: vec![],
    //         }
    //     })
    //     .collect::<Vec<TransactionEntityChanges>>();
    let changes = vec![];
    Ok(BlockChanges {
        block: if !changes.is_empty() { Some((&block).into()) } else { None },
        changes,
    })
}
