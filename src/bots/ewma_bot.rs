use async_trait::async_trait;
use log::{debug, error, info, warn};
use tokio::sync::{broadcast, mpsc};

use crate::bots::Bot;
use crate::manifold_types;
use crate::market_handler;

struct EWMA {
    s0: f64,
    alpha: f64,
}

impl EWMA {
    fn new(s0: f64, alpha: f64) -> Self {
        Self { s0, alpha }
    }

    fn update(&mut self, x: f64) -> f64 {
        self.s0 = self.alpha * x + (1.0 - self.alpha) * self.s0;
        self.s0
    }

    fn get(&self) -> f64 {
        self.s0
    }
}

pub struct EWMABot {
    id: String,
    market: manifold_types::FullMarket,

    bot_to_mh_tx: mpsc::Sender<market_handler::InternalPacket>,
    mh_to_bot_rx: broadcast::Receiver<market_handler::InternalPacket>,

    ewma_1: EWMA,
    ewma_2: EWMA,

    // used as a sanity check
    current_probability: f64,
    p1_above_p2: bool,
}

impl EWMABot {
    pub fn new(
        id: String,
        market: manifold_types::FullMarket,
        bot_to_mh_tx: mpsc::Sender<market_handler::InternalPacket>,
        mh_to_bot_rx: broadcast::Receiver<market_handler::InternalPacket>,
        alpha_1: f64,
        alpha_2: f64,
    ) -> Self {
        let ewma_1 = EWMA::new(0.0, alpha_1);
        let ewma_2 = EWMA::new(0.0, alpha_2);

        Self {
            id,
            market,
            bot_to_mh_tx,
            mh_to_bot_rx,
            ewma_1,
            ewma_2,
            current_probability: 0.0,
            p1_above_p2: true,
        }
    }

    async fn make_trades(&mut self, trades: Vec<market_handler::InternalPacket>) {
        for trade in trades {
            self.bot_to_mh_tx
                .send(trade)
                .await
                .unwrap();

            match self.mh_to_bot_rx.recv().await {
                Ok(resp) => {
                    info!("made bet {:?}", resp);
                }
                Err(e) => {
                    error!("mh_to_bot_rx gave error {e}");
                }
            }
        }
    }

    fn update_prob(&mut self, bet: &manifold_types::Bet) -> manifold_types::Side {
        if self.current_probability != bet.prob_before {
            warn!(
                "bot's current_probability ({}) is not prob_before ({})",
                self.current_probability, bet.prob_after
            );
        }

        self.current_probability = bet.prob_after;

        let v1 = self.ewma_1.update(bet.prob_before);
        let v2 = self.ewma_2.update(bet.prob_after);

        if v1 < v2 && self.p1_above_p2 {
            self.p1_above_p2 = false;
            manifold_types::Side::Sell
        } else if v1 > v2 && !self.p1_above_p2 {
            self.p1_above_p2 = true;
            manifold_types::Side::Buy
        } else {
            manifold_types::Side::NoOp
        }
    }
}

#[async_trait]
impl Bot for EWMABot {
    async fn run(&mut self, mut rx: broadcast::Receiver<manifold_types::Bet>) {
        info!("starting arbitrage bot");

        let mut i: u64 = 0;
        loop {
            let bet: manifold_types::Bet = match rx.recv().await {
                Ok(bet) => bet,
                Err(e) => {
                    warn!("in EWMABot::run {e}");
                    continue;
                }
            };

            debug!("{i} {:?}", bet);

            match self.update_prob(&bet) {
                manifold_types::Side::Buy => {
                    let buy_bet = market_handler::InternalPacket::new(
                        self.get_id(),
                        market_handler::Method::Post,
                        "bet".to_string(),
                        vec![],
                        Some(serde_json::json!({
                            "amount": bet.amount,
                            "contractId": bet.contract_id,
                            "outcome": bet.outcome,
                        })),
                    );

                    self.make_trades(vec![buy_bet]).await;
                },
                manifold_types::Side::Sell => {
                    let sell_bet = market_handler::InternalPacket::new(
                        self.get_id(),
                        market_handler::Method::Post,
                        format!("market/{}/sell", bet.contract_id),
                        vec![],
                        Some(serde_json::json!({
                            "outcome": bet.outcome,
                            "shares": bet.amount
                        }))
                    );

                    self.make_trades(vec![sell_bet]).await;
                },
                manifold_types::Side::NoOp => {}
            }

            i += 1;
        }
    }

    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn close(&self) {
        println!("closing arbitrage bot");
    }
}
