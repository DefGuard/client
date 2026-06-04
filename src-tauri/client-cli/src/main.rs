use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::EnvFilter;

mod cli;
mod commands;
mod exit;
mod output;
mod state;

use cli::Cli;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // Init logging to stderr so stdout stays data-only.
    let log_filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(log_filter)
        .init();

    // Resolve state (data-dir, DB pool, migrations).
    let st = match state::init(cli.data_dir.as_deref()).await {
        Ok(s) => s,
        Err(err) => {
            let code = exit::exit_code_for(&err);
            output::emit_error(&err, cli.json);
            return ExitCode::from(code);
        }
    };

    // Dispatch command.
    let result = match cli.command {
        cli::Commands::List => commands::list::handle(&st, cli.json).await,
        cli::Commands::Status => commands::status::handle(&st, cli.json).await,
        cli::Commands::Connect { .. }
        | cli::Commands::Disconnect { .. }
        | cli::Commands::Location { .. }
        | cli::Commands::Instance { .. }
        | cli::Commands::Tunnel { .. }
        | cli::Commands::Enroll { .. } => {
            // These commands are implemented in later phases.
            let err = state::CliError::Usage("command not yet implemented".into());
            output::emit_error(&err, cli.json);
            return ExitCode::from(exit::exit_code_for(&err));
        }
    };

    match result {
        Ok(()) => ExitCode::from(0),
        Err(err) => {
            let code = exit::exit_code_for(&err);
            output::emit_error(&err, cli.json);
            ExitCode::from(code)
        }
    }
}
