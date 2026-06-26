use std::process::ExitCode;

use clap::Parser;
use common::check_version_flag;

mod brand;
mod cli;
mod commands;
mod config_poll;
mod exit;
mod logging;
mod mfa;
mod mfa_code;
mod mfa_qr;
mod output;
mod resolve;
mod state;
#[cfg(all(test, target_os = "linux"))]
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
    // Handle --version / -V before any other work.
    check_version_flag("defguard-cli");

    // Brand banner: shown before clap's --help, and when invoked with
    // zero arguments. NOT shown for --version (must stay grep-friendly).
    show_banner_if_appropriate();

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

    config_poll::poll_config(&state).await;

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
            qr_file,
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
                qr_file.as_deref(),
                all_traffic,
                predefined_traffic,
                cli.json,
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
    }
}

/// Show the brand banner (logo + copyright + project version) on
/// the two surfaces that need branding: `defguard-cli` with no args
/// (clap prints help; we banner first), and `defguard-cli --help` /
/// `-h`. Suppressed for `--version` / `-V` -- that surface must stay
/// grep-friendly (`defguard-cli --version | head -1`).
fn show_banner_if_appropriate() {
    let args = std::env::args().collect::<Vec<_>>();
    // Skip argv[0]. If user supplied any subcommand or flag other
    // than --help / -h, do not print the banner.
    let user_args = &args[1..];
    let no_args = user_args.is_empty();
    let asked_help = user_args.iter().any(|a| a == "--help" || a == "-h");
    let asked_version = user_args.iter().any(|a| a == "--version" || a == "-V");
    if asked_version {
        return;
    }
    if no_args || asked_help {
        brand::print_banner();
    }
}
