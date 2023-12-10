use crate::manifold_types;

trait Bot {
    fn run(&self);
    fn close(&self);
}

pub struct ArbitrageBot {
    contract_id: String,
    market: manifold_types::FullMarket,
}

impl Bot for ArbitrageBot {
    fn run(&self) {
        println!("running arbitrage bot");
    }

    fn close(&self) {
        println!("closing arbitrage bot");
    }
}
