#![allow(dead_code)]

use crate::storage::utils::StorageLocation;
use substreams::hex;

pub const OWNER: StorageLocation = StorageLocation {
    name: "owner",
    slot: hex!("0000000000000000000000000000000000000000000000000000000000000000"),
    offset: 0,
    number_of_bytes: 20,
};

pub const PAUSED: StorageLocation = StorageLocation {
    name: "paused",
    slot: hex!("0000000000000000000000000000000000000000000000000000000000000000"),
    offset: 20,
    number_of_bytes: 1,
};

pub const ROUTERS: StorageLocation = StorageLocation {
    name: "routers",
    slot: hex!("0000000000000000000000000000000000000000000000000000000000000006"),
    offset: 0,
    number_of_bytes: 32,
};
