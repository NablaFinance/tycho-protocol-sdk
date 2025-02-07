#![allow(dead_code)]
use substreams::scalar::BigInt;
use substreams_ethereum::pb::eth::v2::StorageChange;
use tiny_keccak::{Hasher, Keccak};

#[derive(Clone, Debug)]
pub enum StorageType {
    Address,
    Bool,
    Uint256,
    Array { item_type: &'static StorageType },
    Mapping { key_type: &'static StorageType, value_type: &'static StorageType },
}

impl StorageType {
    pub fn base_type(&self) -> &StorageType {
        match self {
            StorageType::Address => self,
            StorageType::Bool => self,
            StorageType::Uint256 => self,
            StorageType::Array { item_type } => item_type.base_type(),
            StorageType::Mapping { value_type, .. } => value_type.base_type(),
        }
    }

    pub fn number_of_bytes(&self) -> usize {
        match self {
            StorageType::Address => 20,
            StorageType::Bool => 1,
            StorageType::Uint256 => 32,
            StorageType::Array { .. } => 32,
            StorageType::Mapping { .. } => 32,
        }
    }
}

#[derive(Clone)]
pub struct StorageLocation<'a> {
    pub name: &'a str,
    pub storage_type: StorageType,
    pub slot: [u8; 32],
    pub offset: usize,
}

pub fn read_bytes(buf: &[u8], offset: usize, number_of_bytes: usize) -> &[u8] {
    let buf_length = buf.len();
    if buf_length < number_of_bytes {
        panic!(
            "attempting to read {number_of_bytes} bytes in buffer  size {buf_size}",
            number_of_bytes = number_of_bytes,
            buf_size = buf.len()
        )
    }

    if offset > (buf_length - 1) {
        panic!(
            "offset {offset} exceeds buffer size {buf_size}",
            offset = offset,
            buf_size = buf.len()
        )
    }

    let end = buf_length - 1 - offset;
    let start_opt = (end + 1).checked_sub(number_of_bytes);
    if start_opt.is_none() {
        panic!(
            "number of bytes {number_of_bytes} with offset {offset} exceeds buffer size
{buf_size}",
            number_of_bytes = number_of_bytes,
            offset = offset,
            buf_size = buf.len()
        )
    }
    let start = start_opt.unwrap();

    &buf[start..=end]
}

fn keccak256(input: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    let mut output = [0u8; 32];
    hasher.update(input);
    hasher.finalize(&mut output);
    output
}

pub fn compute_element_slot(slot: &[u8], new_length: &[u8]) -> BigInt {
    BigInt::from_unsigned_bytes_be(&keccak256(slot)) + BigInt::from_unsigned_bytes_be(new_length)
        - BigInt::one()
}

pub fn compute_mapping_key(key: &[u8; 32], slot: &[u8; 32]) -> [u8; 32] {
    let mut input = [0u8; 64];
    input[0..32].copy_from_slice(key);
    input[32..64].copy_from_slice(slot);
    keccak256(&input)
}

pub fn pad_address(addr: &[u8]) -> [u8; 32] {
    let mut padded = [0u8; 32];
    padded[12..].copy_from_slice(addr);
    padded
}

pub fn read_item_at_slot(
    element_slot: BigInt,
    storage_changes: &[StorageChange],
    storage_type: &StorageType,
) -> Result<Vec<u8>, String> {
    storage_changes
        .iter()
        .find(|change| BigInt::from_unsigned_bytes_be(&change.key) == element_slot)
        .map(|inner_change| {
            let number_of_bytes = storage_type.number_of_bytes();
            read_bytes(&inner_change.new_value, 0, number_of_bytes).to_vec()
        })
        .ok_or_else(|| format!("Failed to find new element for slot: {}", element_slot))
}
