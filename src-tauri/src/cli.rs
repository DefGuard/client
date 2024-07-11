use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
pub struct DefguardCli {
    #[clap(short, long)]
    pub instances: bool,
    #[clap(short, long, value_parser, num_args = 0.., value_delimiter = ' ')]
    pub vpns: Vec<String>,
    #[clap(short, long, default_value = "")]
    pub connect: String,
    #[clap(short, long, value_parser, num_args = 0.., value_delimiter = ' ')]
    pub disconnect: Vec<String>,
    #[clap(short, long)]
    pub status: bool,
}
