use anyhow::Result;
use substreams_helper::hex::Hexable;
use tycho_substreams::prelude::EntityChanges;

pub fn read_state(router_address: Vec<u8>) -> Result<EntityChanges, String> {
    let router_id = router_address.to_hex();
    Ok(EntityChanges { component_id: router_id.into(), attributes: vec![] })
}
