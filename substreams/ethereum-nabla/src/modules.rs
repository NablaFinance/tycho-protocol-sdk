use anyhow::Result;
use substreams_ethereum::pb::eth;
use tycho_substreams::prelude::{
    BlockTransactionProtocolComponents, TransactionProtocolComponents,
};

#[substreams::handlers::map]
pub fn map_components(block: eth::v2::Block) -> Result<BlockTransactionProtocolComponents> {
    Ok(BlockTransactionProtocolComponents {
        tx_components: block
            .transactions()
            .filter_map(|tx| {
                let components = tx
                    .logs_with_calls()
                    .filter_map(|(_log, _call)| {
                        // println!("Log data: {:?}", log.data);
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

    use prost::Message;

    use std::fs::File;
    use std::io::Read;

    #[test]
    fn test_map_components() {
        let file_path = "../../../firehose-data/storage/merged-blocks/0266195200.bin";
        let mut file = File::open(file_path).expect("Failed to open merged-blocks file");
        let mut block_data = Vec::new();
        file.read_to_end(&mut block_data)
            .expect("Failed to read merged-blocks file");

        let block = Block::decode(&*block_data).expect("Failed to decode");

        if let Some(payload) = block.payload {
            // Confirm the type_url matches the expected type
            if payload.type_url == "type.googleapis.com/sf.ethereum.type.v2.Block" {
                // Decode the raw `value` field into an Ethereum block
                let eth_block = eth::v2::Block::decode(&*payload.value)
                    .expect("Failed to decode Ethereum block");
                println!("Decoded Ethereum Block: {:?}", eth_block);
            } else {
                println!("Unexpected payload type_url: {}", payload.type_url);
            }
        } else {
            println!("No payload found in the block");
        }
    }
}
