use clap::Parser;
use clap::{AppSettings, Arg, ArgMatches};

use std::fs::File;
use std::path::PathBuf;

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
    let input_paths = cli.paths.iter().map(PathBuf::from).collect::<Vec<_>>();

    validate_input_paths(&input_paths)
}

/// check that all input paths are present/valid, if not, terminate
fn validate_input_paths(input_paths: &[PathBuf]) {
    let invalid_paths = input_paths
        .iter()
        .filter(|path| !path.exists())
        .collect::<Vec<_>>();
    if !invalid_paths.is_empty() {
        eprintln!("Warning: the following files could not be found:");
        invalid_paths
            .iter()
            .for_each(|p| eprint!("{}, ", p.display()));
        eprintln!();
        std::process::exit(1);
    }
}