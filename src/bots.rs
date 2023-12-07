trait Bot {
    fn run(&self);
    fn close(&self);
}

pub struct ArbitrageBot {}

impl Bot for ArbitrageBot {
    fn run(&self) {
        println!("running arbitrage bot");
    }

    fn close(&self) {
        println!("closing arbitrage bot");
    }
}
