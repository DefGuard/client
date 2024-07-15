use anyhow::{Error, Result};
use std::path::PathBuf;

use defguard_client::{
    cli::DefguardCli,
    commands::gen_list_of_all_instances,
    database::{self, DB_NAME},
};
use log::{debug, info};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut writer = &mut std::io::stdout().lock();
    env_logger::init();
    let def_cli = DefguardCli::new();

    // TODO: trigger help command and close program if there are no set any of command in [`CliHandler`]

    // Setup database
    // TODO: generate an appropriate path for db instance
    let db_path = PathBuf::from(format!("./{}", DB_NAME));
    let db = database::setup_db(db_path)
        .await
        .expect("Database initialization failed");
    *def_cli.app_state.db.lock().unwrap() = Some(db);

    info!("Database initialization completed");
    info!("Starting main app thread.");
    let result = database::info(&def_cli.app_state.get_pool()).await;
    info!("Database info result: {:#?}", result);

    run_cli_app(def_cli, writer).await
}

pub async fn run_cli_app(
    def_cli: DefguardCli,
    writer: &mut impl std::io::Write,
) -> Result<(), Error> {
    let instances = def_cli.cli.instances;
    match instances {
        true => {
            info!("Listing all instances");
            let instances = gen_list_of_all_instances(&def_cli.app_state)
                .await
                .unwrap_or_default();
            let json = serde_json::to_string(&instances).unwrap();

            writeln!(writer, "{:?}", json).expect("Unable to display instances");
        }
        false => todo!(),
    }

    let disconnect = def_cli.cli.disconnect;
    match disconnect {
        Some(vpn_names) if (vpn_names.is_empty()) => {
            info!("Starting disconnecting from all VPNS");
            let _ = def_cli.app_state.close_all_connections();
        }
        Some(vpn_names) => {
            debug!("vpn names: {:?}", vpn_names);
        }
        None => debug!("option --disconnect was not set"),
    }

    Ok(())
}
