use clap::Parser;
use clap::{AppSettings, Arg, ArgMatches};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    debug: bool,

    #[clap(short, long, default_value_t = 1)]
    threshold: u8,

    #[clap(short, long, default_value_t = 0)]
    jobs: u8,

    paths: Vec<String>,
}

fn main() {
    let cli = Args::parse();
    let input_paths = cli.paths;

    dbg!(input_paths);
}
