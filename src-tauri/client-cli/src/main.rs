use std::process::ExitCode;

use clap::Parser;

mod cli;
mod commands;
mod exit;
mod logging;
mod mfa;
mod mfa_code;
mod output;
mod resolve;
mod state;

use cli::{Cli, LocationCommand};

use crate::{
    cli::Commands,
    commands::{connect, disconnect, list, location, status},
    state::State,
};

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // Init logging to stderr so stdout stays data-only.
    logging::init(cli.verbose);

    // Resolve state (data-dir, DB pool, migrations).
    let state = match State::init(cli.data_dir.as_deref()).await {
        Ok(s) => s,
        Err(err) => {
            let code = exit::exit_code_for(&err);
            output::emit_error(&err, cli.json);
            return ExitCode::from(code);
        }
    };

    // Dispatch command.
    let result = match cli.command {
        Commands::List => list::handle(&state, cli.json).await,
        Commands::Status => status::handle(&state, cli.json).await,
        Commands::Connect {
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
            connect::handle(
                &state,
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
        Commands::Disconnect {
            name,
            tunnel,
            id,
            instance,
            all,
        } => {
            disconnect::handle(
                &state,
                cli.json,
                name.as_deref(),
                tunnel,
                id,
                instance.as_deref(),
                all,
            )
            .await
        }
        Commands::Location(sub) => match sub {
            LocationCommand::List => location::handle_list(&state, cli.json).await,
            LocationCommand::Set {
                name,
                instance,
                mfa_method,
                route_all_traffic,
                no_route_all_traffic,
            } => {
                location::handle_set(
                    &state,
                    cli.json,
                    &name,
                    instance.as_deref(),
                    mfa_method.as_deref(),
                    if route_all_traffic { Some(true) } else { None },
                    no_route_all_traffic,
                )
                .await
            }
            LocationCommand::Show { name, instance } => {
                location::handle_show(&state, cli.json, &name, instance.as_deref()).await
            }
        },
        Commands::Instance { .. } | Commands::Tunnel { .. } | Commands::Enroll { .. } => {
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
