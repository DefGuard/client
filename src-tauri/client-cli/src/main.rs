use std::process::ExitCode;

use clap::Parser;

mod cli;
mod commands;
mod exit;
mod logging;
mod output;
mod resolve;
mod state;

use cli::Cli;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // Init logging to stderr so stdout stays data-only.
    // Quiet (WARN) by default; -v for INFO, -vv for DEBUG, -vvv for TRACE.
    logging::init(cli.verbose);

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
        cli::Commands::Connect {
            name,
            tunnel,
            id,
            instance,
            code,
            code_command,
            mfa_method: _mfa_method,
            all_traffic: _all_traffic,
            no_all_traffic: _no_all_traffic,
        } => {
            commands::connect::handle(
                &st,
                cli.json,
                name.as_deref(),
                tunnel,
                id,
                instance.as_deref(),
                code.as_deref(),
                code_command.as_deref(),
                None,
                false,
                false,
            )
            .await
        }
        cli::Commands::Disconnect {
            name,
            tunnel,
            id,
            instance,
            all,
        } => {
            commands::disconnect::handle(
                &st,
                cli.json,
                name.as_deref(),
                tunnel,
                id,
                instance.as_deref(),
                all,
            )
            .await
        }
        cli::Commands::Location { .. }
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
