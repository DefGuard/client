//! CLI logging — three-stream model: stdout = data only, stderr = diagnostics.
//!
//! Installs a `tracing_subscriber` that writes to **stderr**.  The `tracing-log`
//! feature on `tracing-subscriber` bridges `core`'s `log::*` output so library
//! diagnostics never pollute stdout.
//!
//! Default level: WARN (quiet).  Staged with `-v`/`-vv`/`-vvv`.  `DG_LOG` or
//! `RUST_LOG` in the environment take precedence.

use tracing_subscriber::EnvFilter;

/// Initialise the logging subscriber.
///
/// * `verbosity` — 0 = WARN (quiet), 1 = INFO, 2 = DEBUG, 3+ = TRACE.
/// * If `DG_LOG` or `RUST_LOG` is set in the environment, it takes precedence.
pub fn init(verbosity: u8) {
    let default_directive = match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    let filter = if let Ok(env) = std::env::var("DG_LOG").or_else(|_| std::env::var("RUST_LOG")) {
        EnvFilter::new(env)
    } else {
        EnvFilter::new(default_directive)
    };

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(filter)
        .with_target(false)
        .init();
}
