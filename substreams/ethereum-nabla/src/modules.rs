#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
use crate::abi;
use anyhow::Result;
use substreams::hex;
use substreams_ethereum::{pb::eth, Event};
use tycho_substreams::prelude::{
    BlockTransactionProtocolComponents, ProtocolComponent, TransactionProtocolComponents,
};

pub const PORTAL_CONTRACT: [u8; 20] = hex!("cB94Eee869a2041F3B44da423F78134aFb6b676B");

fn is_deployment_tx(tx: &eth::v2::TransactionTrace, vault_address: &[u8]) -> bool {
    let created_accounts = tx
        .calls
        .iter()
        .flat_map(|call| {
            call.account_creations
                .iter()
                .map(|ac| ac.account.to_owned())
        })
        .collect::<Vec<_>>();

    if let Some(deployed_address) = created_accounts.first() {
        return deployed_address.as_slice() == vault_address;
    }
    false
}

// #[substreams::handlers::map]
pub fn map_components(block: eth::v2::Block) -> Result<BlockTransactionProtocolComponents> {
    Ok(BlockTransactionProtocolComponents {
        tx_components: block
            .transactions()
            .filter_map(|tx| {
                if is_deployment_tx(tx, &PORTAL_CONTRACT) {
                    println!("Block number: {:?}", block.number);
                    println!("CONTRACT FOUND: {:?}", PORTAL_CONTRACT);
                }
                // detect the registered asset events for portal
                let components = tx
                    .logs_with_calls()
                    .filter_map(|(log, call)| {
                        let address = log.address.as_slice();
                        if address == PORTAL_CONTRACT {
                            println!("Block number: {:?}", block.number);
                            println!("{:?}", log);
                            let asset_registered =
                                abi::nabla_portal::events::AssetRegistered::match_and_decode(log)?;
                            println!("Registered Asset: {:?}", asset_registered);
                        }

                        None
                    })
                    .collect::<Vec<_>>();

                (!components.is_empty())
                    .then(|| TransactionProtocolComponents { tx: Some(tx.into()), components })
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::sf::bstream::v1::Block;
    use crate::{abi, block_reader};

    use prost::Message;
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;

    #[test]
    fn test_map_components() {
        let merged_dir = "../../../firehose-data/storage/merged-blocks";
        let block_processor = block_reader::BlockProcessor::new(merged_dir)
            .expect("Failed to initialize BlockProcessor");

        for block in block_processor.take(10_000) {
            if block.number % 1_000 == 0 {
                println!("block: {:?}", block.number);
            }
            let _ = map_components(block);
        }
    }
}
