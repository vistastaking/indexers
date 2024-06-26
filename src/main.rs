mod db;
mod handlers;

use ghost_crab::prelude::*;
// use handlers::stakewise;
// use handlers::rocketpool;
use handlers::etherfi;

#[tokio::main]
async fn main() {
    let mut indexer = ghost_crab::Indexer::new();

    // indexer.load(stakewise::VaultsRegistry::new()).await;
    // indexer.load(rocketpool::MinipoolCreated::new()).await;
    indexer.load(etherfi::EtherFiTVLUpdated::new()).await;

    indexer.start().await;
}
