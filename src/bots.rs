trait Bot {
    fn run(&self);
}

pub struct ArbitrageBot {}

impl Bot for ArbitrageBot {
    fn run(&self) {
        println!("running arbitrage bot");
    }
}
