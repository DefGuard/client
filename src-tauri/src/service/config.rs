use clap::Parser;

#[cfg(windows)]
pub const DEFAULT_LOG_DIR: &str = "/Logs/defguard-service";
#[cfg(not(windows))]
pub const DEFAULT_LOG_DIR: &str = "/var/log/defguard-service";

#[derive(Debug, Parser, Clone)]
#[clap(about = "Defguard VPN client interface management service")]
#[command(version)]
pub struct Config {
    /// Configures log level of defguard service logs
    #[arg(long, env = "DEFGUARD_LOG_LEVEL", default_value = "info")]
    pub log_level: String,

    /// Configures logging directory; it is meant for debugging only, so hide it.
    #[arg(long, env = "DEFGUARD_LOG_DIR", default_value = DEFAULT_LOG_DIR, hide = true)]
    pub log_dir: String,

    /// Defines how often (in seconds) interface statistics are sent to defguard client
    #[arg(long, short = 'p', env = "DEFGUARD_STATS_PERIOD", default_value = "10")]
    pub stats_period: u64,
}
