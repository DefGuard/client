use clap::{Parser, Subcommand};

/// Command-line client for the Defguard VPN.
///
/// Shares the same database as the desktop client.
#[derive(Parser)]
#[command(name = "defguard-cli", version, about)]
pub struct Cli {
    /// Output machine-readable JSON instead of human-readable tables.
    #[arg(long, global = true)]
    pub json: bool,

    /// Increase log verbosity (staged: -v INFO, -vv DEBUG, -vvv TRACE).
    /// Diagnostics go to stderr.  Honors DG_LOG / RUST_LOG env.
    #[arg(short = 'v', long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all configured instances, locations, and tunnels.
    List,

    /// Show currently-active VPN connections (live state from the daemon).
    #[command(alias = "s")]
    Status,

    /// Connect to a location or tunnel.
    #[command(alias = "c")]
    Connect {
        /// Location or tunnel name.  If omitted, connects to the sole configured
        /// location (error if ambiguous).
        name: Option<String>,

        /// Connect to a tunnel instead of a location.
        #[arg(long)]
        tunnel: bool,

        /// Target by id (fast path, skips name resolution).
        #[arg(long)]
        id: Option<i64>,

        /// Instance name qualifier when the same location name exists in multiple
        /// instances.
        #[arg(long)]
        instance: Option<String>,

        /// MFA authentication code (TOTP / email).
        #[arg(long)]
        code: Option<String>,

        /// Shell command that prints the MFA code to stdout.  Receives
        /// DG_INSTANCE and DG_LOCATION in its environment.
        #[arg(long)]
        code_command: Option<String>,

        /// Override the persisted MFA method.
        #[arg(long)]
        mfa_method: Option<String>,

        /// Override route-all-traffic for this connection only.
        #[arg(long, overrides_with = "predefined_traffic")]
        all_traffic: bool,

        /// Do not route all traffic (overrides location default).
        #[arg(long, overrides_with = "all_traffic")]
        predefined_traffic: bool,
    },

    /// Disconnect from a location or tunnel.
    #[command(alias = "d")]
    Disconnect {
        /// Location or tunnel name.  If omitted, disconnects the sole active
        /// connection (error if ambiguous).
        name: Option<String>,

        /// Disconnect a tunnel instead of a location.
        #[arg(long)]
        tunnel: bool,

        /// Target by id.
        #[arg(long)]
        id: Option<i64>,

        /// Instance name qualifier.
        #[arg(long)]
        instance: Option<String>,

        /// Disconnect all active connections.
        #[arg(long)]
        all: bool,
    },

    /// Manage locations (view settings, set MFA method, routing).
    #[command(subcommand, alias = "l")]
    Location(LocationCommand),

    /// Manage instances.
    #[command(subcommand, alias = "i")]
    Instance(InstanceCommand),

    /// Manage imported WireGuard tunnels.
    #[command(subcommand, alias = "t")]
    Tunnel(TunnelCommand),

    /// Enroll a new instance (alternative to the desktop enrollment flow).
    // TODO: Hidden until implemented
    #[command(hide = true)]
    Enroll {
        /// Enrollment token.
        #[arg(long, env = "DG_ENROLLMENT_TOKEN")]
        token: String,

        /// Proxy / Edge URL for enrollment.
        #[arg(long, env = "DG_URL")]
        url: String,
    },
}

#[derive(Subcommand)]
pub enum LocationCommand {
    /// List all locations.
    List,

    /// Show details for a location.
    Show {
        name: String,

        #[arg(long)]
        instance: Option<String>,
    },

    /// Persist a per-location preference.
    Set {
        name: String,

        #[arg(long)]
        instance: Option<String>,

        /// Override the MFA method (totp, email, oidc, mobile).
        #[arg(long)]
        mfa_method: Option<String>,

        /// Always route all traffic through this location.
        #[arg(long, overrides_with = "predefined_traffic")]
        route_all_traffic: bool,

        /// Never route all traffic through this location.
        #[arg(long, overrides_with = "route_all_traffic")]
        predefined_traffic: bool,
    },
}

#[derive(Subcommand)]
pub enum InstanceCommand {
    /// List all enrolled instances.
    List,

    /// Show details for an instance.
    Show { name: String },
}

#[derive(Subcommand)]
pub enum TunnelCommand {
    /// List all imported tunnels.
    List,

    /// Show details for a tunnel.
    Show { name: String },
}
