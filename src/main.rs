use clap::Parser;
use humansize::{file_size_opts as options, FileSize};
use walkdir::WalkDir;

use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

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

static exec_optipng: &str = "optipng";
static exec_imagemagic: &str = "convert";
static exec_advdef: &str = "avdef";
static exec_oxipng: &str = "oxipng";

fn main() {
    let cli = Args::parse();
    let input_paths = cli.paths.iter().map(PathBuf::from).collect::<Vec<_>>();

    validate_input_paths(&input_paths);

    let all_png_files = input_paths
        .iter()
        .flat_map(|path| WalkDir::new(path).into_iter().filter_map(|e| e.ok()))
        .map(|f| f.into_path())
        // collect all .png files
        .filter(|file| file.extension() == Some(OsStr::new("png")))
        .collect::<Vec<PathBuf>>();

    let total_file_size_before = all_png_files
        .iter()
        .flat_map(std::fs::metadata)
        .map(|metadata| metadata.len())
        .sum::<u64>();

    println!(
        "Checking {} files of total size: {}",
        all_png_files.len(),
        total_file_size_before
            .file_size(options::CONVENTIONAL)
            .unwrap()
    );

    assert_optimizers_are_available();
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
    } else {
        eprintln!("no <path> argument supplied. try '.' for current directory");
    }
}

// make sure all compression tools are available: optipng, imagemagick/convert, advdef, oxipng
fn assert_optimizers_are_available() {
    let arr = [exec_optipng, exec_imagemagic, exec_advdef, exec_oxipng];
    let bad = arr.iter().find(|exe| {
        let mut cmd = Command::new(exe);
        !matches!(cmd.output().ok().map(|x| x.status.success()), Some(true))
    });
    if let Some(not_found) = bad {
        eprintln!("could not find executable for {}", not_found);
        std::process::exit(2);
    }
}
