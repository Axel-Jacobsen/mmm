use async_trait::async_trait;

use tokio::sync::broadcast;

use crate::manifold_types;

#[async_trait]
pub trait Bot {
    async fn run(&mut self, rx: broadcast::Receiver<manifold_types::Bet>);
    fn get_id(&self) -> String;
    fn close(&self);
}

pub mod arb_bot;
pub mod ewma_bot;
