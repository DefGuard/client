use std::path::PathBuf;

use defguard_client::{
    cli::DefguardCli,
    database::{self, DB_NAME},
};
use log::info;

#[tokio::main]
async fn main() {
    if std::env::var_os("DEFGUARD_CLIENT_LOG_LEVEL").is_none() {
        std::env::set_var("DEFGUARD_CLIENT_LOG_LEVEL", "debug");
    }
    env_logger::init();

    let def_cli = DefguardCli::new();

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
    println!("Database info result: {:#?}", result);
}
