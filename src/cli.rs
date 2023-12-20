use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run the bot
    Run {
        /// Set the log level (e.g., "info", "warn", "error")
        #[arg(short, long, default_value_t = String::from("info"))]
        pub log_level: String,

        /// Path to the configuration file
        #[arg(short, long)]
        pub config: Option<String>,
    },

    /// Print the current version
    Liquidate {},
}
