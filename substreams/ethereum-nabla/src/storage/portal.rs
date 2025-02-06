use crate::storage::utils::{StorageLocation, StorageType};
use substreams::hex;

pub const OWNER: StorageLocation = StorageLocation {
    name: "owner",
    storage_type: StorageType::Address,
    slot: hex!("0000000000000000000000000000000000000000000000000000000000000000"),
    offset: 0,
};

pub const PAUSED: StorageLocation = StorageLocation {
    name: "paused",
    storage_type: StorageType::Bool,
    slot: hex!("0000000000000000000000000000000000000000000000000000000000000000"),
    offset: 20,
};

pub const GATE: StorageLocation = StorageLocation {
    name: "gate",
    storage_type: StorageType::Address,
    slot: hex!("0000000000000000000000000000000000000000000000000000000000000002"),
    offset: 0,
};

pub const GATED: StorageLocation = StorageLocation {
    name: "gated",
    storage_type: StorageType::Bool,
    slot: hex!("0000000000000000000000000000000000000000000000000000000000000002"),
    offset: 20,
};

pub const ORACLE_ADAPTER: StorageLocation = StorageLocation {
    name: "oracle_adapter",
    storage_type: StorageType::Address,
    slot: hex!("0000000000000000000000000000000000000000000000000000000000000003"),
    offset: 0,
};

pub const GUARD_ORACLE: StorageLocation = StorageLocation {
    name: "oracle_adapter",
    storage_type: StorageType::Address,
    slot: hex!("0000000000000000000000000000000000000000000000000000000000000004"),
    offset: 0,
};

pub const ROUTERS: StorageLocation = StorageLocation {
    name: "routers",
    storage_type: StorageType::Array { item_type: &StorageType::Address },
    slot: hex!("0000000000000000000000000000000000000000000000000000000000000006"),
    offset: 0,
};
