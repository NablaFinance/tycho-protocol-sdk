use crate::modules::map_components::ComponentType;
use crate::pb::nabla::v1::{component::Kind, Component, Portal, Router, SwapPool};
use substreams::store::{StoreNew, StoreSetIfNotExists, StoreSetIfNotExistsProto};
use tycho_substreams::prelude::{BlockTransactionProtocolComponents, ProtocolComponent};

impl From<ProtocolComponent> for Component {
    fn from(pc: ProtocolComponent) -> Self {
        let protocol_type = pc
            .protocol_type
            .expect("ProtocolType not set")
            .name;
        let kind = match protocol_type.into() {
            ComponentType::Portal => Kind::Portal(Portal { address: pc.id.into() }),
            ComponentType::Router => Kind::Router(Router { address: pc.id.into() }),
            ComponentType::SwapPool => Kind::SwapPool(SwapPool { address: pc.id.into() }),
        };
        Component { kind: Some(kind) }
    }
}

#[substreams::handlers::store]
pub fn store_components(
    components: BlockTransactionProtocolComponents,
    store: StoreSetIfNotExistsProto<Component>,
) {
    components
        .tx_components
        .into_iter()
        .for_each(|tx_pc| {
            tx_pc
                .components
                .into_iter()
                .for_each(|pc| store.set_if_not_exists(0, pc.id.clone(), &pc.into()))
        });
}
