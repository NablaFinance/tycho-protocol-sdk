use crate::abi;
use anyhow::Result;
use substreams_helper::hex::Hexable;
use tycho_substreams::prelude::{Attribute, ChangeType, EntityChanges};

pub fn read_state(pool_address: Vec<u8>) -> Result<EntityChanges, String> {
    let pool_id = pool_address.to_hex();

    let (reserves, liabilities) = abi::swap_pool::functions::Coverage {}
        .call(pool_address.clone())
        .ok_or_else(|| format!("Failed to retrieve 'coverage' for pool {:?}", pool_id))?;

    let pool_cap = abi::swap_pool::functions::PoolCap {}
        .call(pool_address.clone())
        .ok_or_else(|| format!("Failed to retrieve 'pool_cap' for pool {:?}", pool_id))?;

    let max_coverage_ratio_for_swap_in = abi::swap_pool::functions::MaxCoverageRatioForSwapIn {}
        .call(pool_address.clone())
        .ok_or_else(|| {
            format!("Failed to retrieve 'max_coverage_ratio_for_swap_in' for pool {:?}", pool_id)
        })?;

    let paused = abi::swap_pool::functions::Paused {}
        .call(pool_address.clone())
        .ok_or_else(|| format!("Failed to retrieve 'paused' for pool {:?}", pool_id))?;

    let is_gated = abi::swap_pool::functions::IsGated {}
        .call(pool_address.clone())
        .ok_or_else(|| format!("Failed to retrieve 'is_gated' for pool {:?}", pool_id))?;

    Ok(EntityChanges {
        component_id: pool_id.into(),
        attributes: vec![
            Attribute {
                name: "reserves".to_string(),
                value: reserves.to_signed_bytes_be(),
                change: ChangeType::Creation.into(),
            },
            Attribute {
                name: "liabilities".to_string(),
                value: liabilities.to_signed_bytes_be(),
                change: ChangeType::Creation.into(),
            },
            Attribute {
                name: "pool_cap".to_string(),
                value: pool_cap.to_signed_bytes_be(),
                change: ChangeType::Creation.into(),
            },
            Attribute {
                name: "max_coverage_ratio_for_swap_in".to_string(),
                value: max_coverage_ratio_for_swap_in.to_signed_bytes_be(),
                change: ChangeType::Creation.into(),
            },
            Attribute {
                name: "paused".to_string(),
                value: vec![paused as u8],
                change: ChangeType::Creation.into(),
            },
            Attribute {
                name: "is_gated".to_string(),
                value: vec![is_gated as u8],
                change: ChangeType::Creation.into(),
            },
        ],
    })
}
