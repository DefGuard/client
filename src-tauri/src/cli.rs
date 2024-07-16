use clap::Parser;

use crate::appstate::AppState;

// TODO: add description
#[derive(Parser)]
#[command(version, about)]
pub struct CliHandler {
    #[clap(short, long)]
    pub instances: bool,
    #[clap(short, long, value_parser, num_args = 0.., value_delimiter = ' ')]
    pub vpns: Option<Vec<String>>,
    #[clap(short, long)]
    pub connect: Option<String>,
    #[clap(short, long, value_parser, num_args = 0.., value_delimiter = ' ')]
    pub disconnect: Option<Vec<String>>,
    #[clap(short, long)]
    pub status: bool,
}

pub struct DefguardCli {
    pub cli: CliHandler,
    pub app_state: AppState,
}

impl Default for DefguardCli {
    fn default() -> Self {
        Self::new()
    }
}

impl DefguardCli {
    pub fn new() -> Self {
        DefguardCli {
            cli: CliHandler::parse(),
            app_state: AppState::new(),
        }
    }
}
