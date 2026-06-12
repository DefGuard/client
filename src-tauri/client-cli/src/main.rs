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

use cli::{Cli, InstanceCommand, LocationCommand, TunnelCommand};

use crate::{
    cli::Commands,
    commands::{connect, disconnect, instance, list, location, status, tunnel},
    state::State,
};

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // Init logging to stderr so stdout stays data-only.
    logging::init(cli.verbose);

    // Resolve state (DB pool, migrations, app config).
    let state = match State::init().await {
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
        } => output::finish(
            disconnect::handle(
                &state,
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
            LocationCommand::List => output::finish(location::handle_list(&state).await, cli.json),
            LocationCommand::Set {
                name,
                instance,
                mfa_method,
                route_all_traffic,
                predefined_traffic,
            } => output::finish(
                location::handle_set(
                    &state,
                    &name,
                    instance.as_deref(),
                    mfa_method.as_deref(),
                    if route_all_traffic { Some(true) } else { None },
                    predefined_traffic,
                )
                .await,
                cli.json,
            ),
            LocationCommand::Show { name, instance } => output::finish(
                location::handle_show(&state, &name, instance.as_deref()).await,
                cli.json,
            ),
        },
        Commands::Instance(sub) => match sub {
            InstanceCommand::List => output::finish(instance::handle_list(&state).await, cli.json),
            InstanceCommand::Show { name } => {
                output::finish(instance::handle_show(&state, &name).await, cli.json)
            }
        },
        Commands::Tunnel(sub) => match sub {
            TunnelCommand::List => output::finish(tunnel::handle_list(&state).await, cli.json),
            TunnelCommand::Show { name } => {
                output::finish(tunnel::handle_show(&state, &name).await, cli.json)
            }
        },
        Commands::Enroll { .. } => {
            let err = state::CliError::Usage("command not yet implemented".into());
            let code = exit::exit_code_for(&err);
            output::emit_error(&err, cli.json);
            ExitCode::from(code)
        }
    }
}
