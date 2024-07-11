use clap::Parser;
use defguard_client::cli;

#[tokio::main]
async fn main() {
    let cli_app = cli::DefguardCli::parse();

    println!("instances: {}\nvpns: {:?}", cli_app.instances, cli_app.vpns);
}
