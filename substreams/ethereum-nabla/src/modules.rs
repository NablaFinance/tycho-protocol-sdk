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
                    .filter_map(|(log, _call)| {
                        println!("Log data: {:?}", log.data);
                        None
                    })
                    .collect::<Vec<_>>();

                (!components.is_empty())
                    .then(|| TransactionProtocolComponents { tx: Some(tx.into()), components })
            })
            .collect(),
    })
}
