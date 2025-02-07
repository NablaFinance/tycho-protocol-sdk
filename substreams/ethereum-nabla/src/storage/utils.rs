#![allow(dead_code)]

#[derive(Clone, Debug)]
pub enum StorageType {
    Address,
    Bool,
    Uint256,
    Array { item_type: &'static StorageType },
    Mapping { key_type: &'static StorageType, value_type: &'static StorageType },
}

impl StorageType {
    pub fn number_of_bytes(&self) -> usize {
        match self {
            StorageType::Address => 20,
            StorageType::Bool => 1,
            StorageType::Uint256 => 32,
            StorageType::Array { .. } => 32,
            StorageType::Mapping { .. } => 32,
        }
    }
    pub fn item_type(&self) -> Result<&StorageType, String> {
        if let StorageType::Array { item_type } = self {
            Ok(item_type)
        } else {
            Err("StorageType is not an Array".into())
        }
    }
    pub fn key_type(&self) -> Result<&StorageType, String> {
        if let StorageType::Mapping { key_type, .. } = self {
            Ok(key_type)
        } else {
            Err("StorageType is not a Mapping".into())
        }
    }
    pub fn value_type(&self) -> Result<&StorageType, String> {
        if let StorageType::Mapping { value_type, .. } = self {
            Ok(value_type)
        } else {
            Err("StorageType is not a Mapping".into())
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
