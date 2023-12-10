use async_trait::async_trait;
use log::{debug, info, warn};
use tokio::sync::broadcast::Receiver;

use crate::manifold_types;

#[async_trait]
pub trait Bot {
    async fn run(&self, rx: Receiver<manifold_types::Bet>);
    fn close(&self);
}

pub struct ArbitrageBot {
    market: manifold_types::FullMarket,
}

impl ArbitrageBot {
    pub fn new(market: manifold_types::FullMarket) -> Self {
        Self { market }
    }
}

#[async_trait]
impl Bot for ArbitrageBot {
    async fn run(&self, mut rx: Receiver<manifold_types::Bet>) {
        info!("starting arbitrage bot");
        let mut i: u64 = 0;
        loop {
            match rx.recv().await {
                Ok(bet) => {
                    debug!("{i} {:?}", bet);
                    i += 1;
                }
                Err(e) => {
                    warn!("{e}");
                }
            }
        }
    }

    fn close(&self) {
        println!("closing arbitrage bot");
    }
}
