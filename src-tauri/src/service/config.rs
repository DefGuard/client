use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(about = "Defguard VPN gateway service")]
#[command(version)]
pub struct Config {
    /// Defines how often (in seconds) interface statistics are sent to Defguard client
    #[arg(long, short = 'p', env = "DEFGUARD_STATS_PERIOD", default_value = "10")]
    pub stats_period: u64,
}
