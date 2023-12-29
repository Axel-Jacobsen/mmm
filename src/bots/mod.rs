

use async_trait::async_trait;

use tokio::sync::{broadcast};

use crate::manifold_types;
use crate::market_handler;

#[async_trait]
pub trait Bot {
    async fn run(&mut self, rx: broadcast::Receiver<manifold_types::Bet>);
    fn get_id(&self) -> String;
    fn close(&self);
    fn botbet_to_internal_coms_packet(
        &self,
        bet: manifold_types::BotBet,
    ) -> market_handler::InternalPacket;
}

pub mod arb_bot;
