use crate::abi;
use anyhow::Result;
use itertools::Itertools;

use substreams::hex;
use substreams_ethereum::pb::eth::v2::{Log, TransactionTrace};
use substreams_ethereum::{pb::eth, Event};
use substreams_helper::hex::Hexable;
use tycho_substreams::prelude::{
    Attribute, BlockTransactionProtocolComponents, ChangeType, FinancialType, ImplementationType,
    ProtocolComponent, ProtocolType, TransactionProtocolComponents,
};

struct Portal {
    contract_address: [u8; 20],
    tx_hash: [u8; 32],
}

const NABLA_PORTAL: Portal = Portal {
    contract_address: hex!("cB94Eee869a2041F3B44da423F78134aFb6b676B"),
    tx_hash: hex!("7e9c5f39e41080aa7ab891ddd8669efc7191bae5906f74c2d5c7e7c12e219046"),
};

#[derive(Debug, Clone)]
pub enum ComponentType {
    Portal,
    Router,
    SwapPool,
}

impl Into<String> for ComponentType {
    fn into(self) -> String {
        match self {
            ComponentType::Portal => "portal".into(),
            ComponentType::Router => "router".into(),
            ComponentType::SwapPool => "swap_pool".into(),
        }
    }
}

impl From<String> for ComponentType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "portal" => ComponentType::Portal,
            "router" => ComponentType::Router,
            "swap_pool" => ComponentType::SwapPool,
            _ => panic!("Unknown protocol type: '{}'", s),
        }
    }
}

fn initialize_portal(tx: &TransactionTrace) -> ProtocolComponent {
    let portal_id = NABLA_PORTAL
        .contract_address
        .to_vec()
        .to_hex();

    ProtocolComponent {
        id: portal_id,
        tokens: vec![],
        tx: Some(tx.into()),
        change: i32::from(ChangeType::Creation),
        contracts: vec![],
        static_att: vec![],
        protocol_type: Option::from(ProtocolType {
            name: ComponentType::Portal.into(),
            financial_type: FinancialType::Swap.into(),
            attribute_schema: vec![],
            implementation_type: ImplementationType::Custom.into(),
        }),
    }
}

fn initialize_router(
    tx: &TransactionTrace,
    router_address: Vec<u8>,
) -> Result<ProtocolComponent, String> {
    let router_id = router_address.to_hex();

    Ok(ProtocolComponent {
        id: router_id,
        tokens: vec![],
        tx: Some(tx.into()),
        change: i32::from(ChangeType::Creation),
        contracts: vec![],
        static_att: vec![],
        protocol_type: Option::from(ProtocolType {
            name: ComponentType::Router.into(),
            financial_type: FinancialType::Swap.into(),
            attribute_schema: vec![],
            implementation_type: ImplementationType::Custom.into(),
        }),
    })
}

fn initialize_swap_pool(
    tx: &TransactionTrace,
    pool_address: Vec<u8>,
) -> Result<ProtocolComponent, String> {
    let pool_id = pool_address.to_hex();

    let asset = abi::swap_pool::functions::Asset {}
        .call(pool_address.clone())
        .ok_or_else(|| format!("Failed to retrieve 'asset' for pool {:?}", pool_id))?;

    let name = abi::swap_pool::functions::Name {}
        .call(pool_address.clone())
        .ok_or_else(|| format!("Failed to retrieve 'name' for pool {:?}", pool_id))?;

    let symbol = abi::swap_pool::functions::Symbol {}
        .call(pool_address.clone())
        .ok_or_else(|| format!("Failed to retrieve 'symbol' for pool {:?}", pool_id))?;

    let attributes = vec![
        Attribute {
            name: "name".to_string(),
            value: name.into(),
            change: ChangeType::Creation.into(),
        },
        Attribute {
            name: "symbol".to_string(),
            value: symbol.into(),
            change: ChangeType::Creation.into(),
        },
    ];

    Ok(ProtocolComponent {
        id: pool_id,
        tokens: vec![asset],
        tx: Some(tx.into()),
        change: i32::from(ChangeType::Creation),
        contracts: vec![],
        static_att: attributes,
        protocol_type: Option::from(ProtocolType {
            name: ComponentType::SwapPool.into(),
            financial_type: FinancialType::Swap.into(),
            attribute_schema: vec![],
            implementation_type: ImplementationType::Custom.into(),
        }),
    })
}

fn detect_asset_registered_event(log: &Log) -> Option<abi::nabla_portal::events::AssetRegistered> {
    (log.address == NABLA_PORTAL.contract_address)
        .then(|| abi::nabla_portal::events::AssetRegistered::match_and_decode(log))?
}

fn to_component(
    tx: &TransactionTrace,
    asset_registered: abi::nabla_portal::events::AssetRegistered,
) -> Result<[ProtocolComponent; 2], String> {
    let router_id = asset_registered.router.to_hex();
    let asset_id = asset_registered.asset.to_hex();

    let pool_address = abi::nabla_router::functions::PoolByAsset { param0: asset_registered.asset }
        .call(asset_registered.router.clone())
        .ok_or_else(|| {
            format!(
                "Failed to retrieve pool_address for asset {} from the router {}",
                asset_id, router_id
            )
        })?;

    let router = initialize_router(tx, asset_registered.router)?;
    let swap_pool = initialize_swap_pool(tx, pool_address.clone())?;
    Ok([router, swap_pool])
}

fn collect_components(
    tx: &TransactionTrace,
) -> Result<Option<TransactionProtocolComponents>, String> {
    let components: Vec<ProtocolComponent>;
    if tx.hash == NABLA_PORTAL.tx_hash {
        components = vec![initialize_portal(tx)];
    } else {
        components = tx
            .logs_with_calls()
            .filter_map(|(log, _call)| detect_asset_registered_event(log))
            .map(|asset_registered| to_component(tx, asset_registered))
            .flatten_ok()
            .try_collect()?;
    }
    Ok((!components.is_empty())
        .then(|| TransactionProtocolComponents { tx: Some(tx.into()), components }))
}

#[substreams::handlers::map]
pub fn map_components(block: eth::v2::Block) -> Result<BlockTransactionProtocolComponents, String> {
    Ok(BlockTransactionProtocolComponents {
        tx_components: block
            .transactions()
            .filter_map(|tx| collect_components(tx).transpose())
            .collect::<Result<Vec<_>, _>>()?,
    })
}
