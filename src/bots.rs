use std::collections::HashMap;

use async_trait::async_trait;
use log::{debug, info, warn};
use tokio::sync::broadcast::Receiver;

use crate::manifold_types;

#[async_trait]
pub trait Bot {
    async fn run(&mut self, rx: Receiver<manifold_types::Bet>);
    fn close(&self);
}

pub struct ArbitrageBot {
    market: manifold_types::FullMarket,
    answers: HashMap<String, manifold_types::Answer>,
}

impl ArbitrageBot {
    pub fn new(market: manifold_types::FullMarket) -> Self {
        let mut id_to_answers = HashMap::new();
        match &market.answers {
            Some(answers) => {
                for answer in answers {
                    id_to_answers.insert(answer.id.clone(), answer.clone());
                }
            }
            None => {
                warn!("market {} has no answers", &market.lite_market.question);
                panic!("market {} has no answers", &market.lite_market.question);
            }
        }
        Self {
            market,
            answers: id_to_answers,
        }
    }

    pub fn find_arb(&self) -> f64 {
        let mut tot_prob: f64 = 0.;
        for answer in self.answers.values() {
            tot_prob += answer.probability;
        }
        tot_prob
    }
}

#[async_trait]
impl Bot for ArbitrageBot {
    async fn run(&mut self, mut rx: Receiver<manifold_types::Bet>) {
        info!("starting arbitrage bot");

        let tot_prob = self.find_arb();
        if tot_prob >= 1. {
            info!("FOUND ARB OPPORTUNITY! {tot_prob}");
        } else {
            info!("NOT ARB OPPORTUNITY {tot_prob}");
        }

        let mut i: u64 = 0;
        loop {
            match rx.recv().await {
                Ok(bet) => {
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

                    i += 1;
                }
                Err(e) => {
                    warn!("in ArbitrageBot::run {e}");
                }
            }
        }
    }

    fn close(&self) {
        println!("closing arbitrage bot");
    }
}
