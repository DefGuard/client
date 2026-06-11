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
#[cfg(test)]
mod tests_daemon;
#[cfg(test)]
mod tests_proxy;

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
    match cli.command {
        Commands::List => output::finish(list::handle(&state).await, cli.json),
        Commands::Status => output::finish(status::handle(&state).await, cli.json),
        Commands::Connect {
            name,
            tunnel,
            id,
            instance,
            code,
            code_command,
            mfa_method,
            all_traffic,
            predefined_traffic,
        } => output::finish(
            connect::handle(
                &state,
                name.as_deref(),
                tunnel,
                id,
                instance.as_deref(),
                code.as_deref(),
                code_command.as_deref(),
                mfa_method.as_deref(),
                all_traffic,
                predefined_traffic,
            )
            .await,
            cli.json,
        ),
        Commands::Disconnect {
            name,
            tunnel,
            id,
            instance,
            all,
        } => output::finish_legacy(
            disconnect::handle(
                &state,
                cli.json,
                name.as_deref(),
                tunnel,
                id,
                instance.as_deref(),
                all,
            )
            .await,
            cli.json,
        ),
        Commands::Location(sub) => match sub {
            LocationCommand::List => {
                output::finish_legacy(location::handle_list(&state, cli.json).await, cli.json)
            }
            LocationCommand::Set {
                name,
                instance,
                mfa_method,
                route_all_traffic,
                no_route_all_traffic,
            } => output::finish_legacy(
                location::handle_set(
                    &state,
                    cli.json,
                    &name,
                    instance.as_deref(),
                    mfa_method.as_deref(),
                    if route_all_traffic { Some(true) } else { None },
                    no_route_all_traffic,
                )
                .await,
                cli.json,
            ),
            LocationCommand::Show { name, instance } => output::finish_legacy(
                location::handle_show(&state, cli.json, &name, instance.as_deref()).await,
                cli.json,
            ),
        },
        Commands::Instance { .. } | Commands::Tunnel { .. } | Commands::Enroll { .. } => {
            let err = state::CliError::Usage("command not yet implemented".into());
            let code = exit::exit_code_for(&err);
            output::emit_error(&err, cli.json);
            ExitCode::from(code)
        }
    }
}
