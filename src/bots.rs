use std::collections::HashMap;

use async_trait::async_trait;
use log::{debug, error, info, warn};
use tokio::sync::{broadcast, mpsc};

use crate::manifold_types;
use crate::market_handler;

#[async_trait]
pub trait Bot {
    async fn run(&mut self, rx: broadcast::Receiver<manifold_types::Bet>);
    fn get_id(&self) -> String;
    fn close(&self);
}

pub struct ArbitrageBot {
    id: String,
    me: manifold_types::User,
    market: manifold_types::FullMarket,
    answers: HashMap<String, manifold_types::Answer>,
    bot_to_mh_tx: mpsc::Sender<market_handler::PostyPacket>,
    mh_to_bot_rx: broadcast::Receiver<market_handler::PostyPacket>,
}

impl ArbitrageBot {
    pub fn new(
        id: String,
        me: manifold_types::User,
        market: manifold_types::FullMarket,
        bot_to_mh_tx: mpsc::Sender<market_handler::PostyPacket>,
        mh_to_bot_rx: broadcast::Receiver<market_handler::PostyPacket>,
    ) -> Self {
        let mut id_to_answers = HashMap::new();

        match &market.answers {
            Some(answers) => {
                for answer in answers {
                    id_to_answers.insert(answer.id.clone(), answer.clone());
                }
            }
            None => {
                error!("market {} has no answers", &market.lite_market.question);
                panic!("market {} has no answers", &market.lite_market.question);
            }
        }

        Self {
            id,
            me,
            market,
            answers: id_to_answers,
            bot_to_mh_tx,
            mh_to_bot_rx,
        }
    }

    pub fn find_arb(&self) -> f64 {
        let mut tot_prob: f64 = 0.;
        for answer in self.answers.values() {
            tot_prob += answer.probability;
        }
        tot_prob
    }

    fn bet_amount(&self) -> f64 {
        let mut bet_map: HashMap<String, manifold_types::BotBet> = HashMap::new();
        let inverse_sum: f64 = self.answers.values().map(|a| 1.0 / a.probability).sum();

        for answer in self.answers.values() {
            let bb = manifold_types::BotBet {
                amount: 100. * (1. / answer.probability) / inverse_sum,
                contract_id: self.market.lite_market.id.clone(),
                outcome: manifold_types::MarketOutcome::Other(answer.id.clone()),
            };
            bet_map.insert(answer.id.clone(), bb);
        }
        info!("BET MAP{:?}", bet_map);

        assert!(
            (bet_map.values().map(|bb| bb.amount).sum::<f64>() - 100.).abs() < 1e-5,
            "sum of bets {} != 100",
            bet_map.values().map(|bb| bb.amount).sum::<f64>()
        );

        0.
    }
}

#[async_trait]
impl Bot for ArbitrageBot {
    async fn run(&mut self, mut rx: broadcast::Receiver<manifold_types::Bet>) {
        info!("starting arbitrage bot");

        let tot_prob = self.find_arb();
        if tot_prob >= 1. {
            info!("FOUND ARB OPPORTUNITY! {tot_prob}");
        } else {
            info!("NOT ARB OPPORTUNITY {tot_prob}");
        }
        self.bet_amount();

        let mut i: u64 = 0;
        loop {
            let bet = match rx.recv().await {
                Ok(bet) => bet,
                Err(e) => {
                    warn!("in ArbitrageBot::run {e}");
                    continue;
                }
            };

            debug!("{i} {:?}", bet);

            let answer_id = &bet.answer_id.expect("answer_id is None");

            debug!(
                "answer_id {answer_id} prob before {} new prob {} our previous prob{}",
                &bet.prob_before,
                &bet.prob_after,
                self.answers.get_mut(answer_id).unwrap().probability
            );

            let bet_prev_prob = &bet.prob_before;
            let bet_after_prob = &bet.prob_after;
            let our_prev_prob = &self.answers.get_mut(answer_id).unwrap().probability;

            if bet_prev_prob != our_prev_prob {
                warn!(
                    "bet_prev_prob {} != our_prev_prob {}",
                    bet_prev_prob, our_prev_prob
                );
            }

            self.answers.get_mut(answer_id).unwrap().probability = *bet_after_prob;

            let tot_prob = self.find_arb();
            if tot_prob >= 1. {
                info!("FOUND ARB OPPORTUNITY! {tot_prob}");
            } else {
                info!("NOT ARB OPPORTUNITY {tot_prob}");
            }

            self.bet_amount();

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
