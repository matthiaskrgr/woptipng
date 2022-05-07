use clap::Parser;
use humansize::{file_size_opts as options, FileSize};
use image::{open, GenericImageView};
use rayon::prelude::*;
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

static EXEC_OPTIPNG: &str = "optipng";
static EXEC_IMAGEMAGIC: &str = "convert";
static EXEC_ADVPNG: &str = "advpng";
static EXEC_OXIPNG: &str = "oxipng";

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

    // optimize
    all_png_files
        .par_iter()
        .map(Image::new)
        .for_each(|mut img| img.optimize());

    let total_file_size_after = all_png_files
        .iter()
        .flat_map(std::fs::metadata)
        .map(|metadata| metadata.len())
        .sum::<u64>();

    println!(
        "Reduced size of  {} files to a total size of: {}",
        all_png_files.len(),
        total_file_size_after
            .file_size(options::CONVENTIONAL)
            .unwrap()
    );

    println!("{}", total_file_size_after - total_file_size_before);
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
    let arr = [EXEC_OPTIPNG, EXEC_IMAGEMAGIC, EXEC_ADVPNG, EXEC_OXIPNG];
    let bad = arr.iter().find(|exe| {
        let mut cmd = Command::new(exe);
        !matches!(
            cmd.arg("--help").output().ok().map(|x| x.status.success()),
            Some(true)
        )
    });
    if let Some(not_found) = bad {
        eprintln!("could not find executable for {}", not_found);
        std::process::exit(2);
    }
}

fn images_are_identical(image1: &PathBuf, image2: &PathBuf) -> bool {
    let image_1 = open(image1).unwrap();
    let pixels_1 = image_1.pixels();

    let image_2 = open(image2).unwrap();
    let pixels_2 = image_2.pixels();

    pixels_1.eq(pixels_2)
}

struct Image<'a> {
    path: &'a PathBuf,
}

impl<'a> Image<'a> {
    fn new(path: &'a PathBuf) -> Self {
        Image { path }
    }
    fn run_imagemagick(&self, tmp_path: &PathBuf) -> bool {
        // copy files
        std::fs::copy(&self.path, tmp_path).expect(&format!(
            "{} to {}",
            &self.path.display(),
            tmp_path.display()
        ));
        let mut cmd = Command::new(EXEC_IMAGEMAGIC);
        cmd.args(["-strip", "-define", "png:color-type=6"])
            .args([self.path, tmp_path]);

        // do not discard output
        cmd.status().unwrap().success()
    }

    fn run_optipng(&self, tmp_path: &PathBuf) -> bool {
        // copy files
        std::fs::copy(&self.path, tmp_path).expect("failed to copy");
        let mut cmd = Command::new(EXEC_OPTIPNG);
        cmd.args(["-q", "-o5", "-nb", "-nc", "-np"]).arg(tmp_path);

        // do not discard output
        cmd.status().unwrap().success()
    }

    fn run_advpng(&self, tmp_path: &PathBuf) -> bool {
        const COMPRESSION_LEVELS: &[u8] = &[1, 2, 3, 4];

        let v = COMPRESSION_LEVELS
            .iter()
            .map(|lvl| {
                let mut cmd = Command::new(EXEC_ADVPNG);
                cmd.arg("-z").arg(format!("-{}", lvl)).arg(tmp_path);
                // discard output
                cmd.output().unwrap().status.success()
            })
            .collect::<Vec<bool>>();

        v.into_iter().all(|v| v)
    }

    fn run_oxipng(&self, tmp_path: &PathBuf) -> bool {
        // copy files
        std::fs::copy(&self.path, tmp_path).expect("failed to copy");
        let mut cmd = Command::new(EXEC_OXIPNG);
        cmd.args(["--nc", "--np", "-o6", "--quiet"]).arg(tmp_path);

        // discard output
        cmd.output().unwrap().status.success()
    }

    fn verify_image(&mut self, backup_image: &PathBuf) {
        let pixel_identical: bool = images_are_identical(self.path, backup_image);

        let size_new = std::fs::metadata(self.path).unwrap().len();
        let size_old = std::fs::metadata(backup_image).unwrap().len();
        let image_got_smaller: bool = size_new < size_old;

        match (pixel_identical, image_got_smaller) {
            (true, true) => {
                // if we got smaller, overwrite backup with smaller version
                std::fs::copy(self.path, backup_image).unwrap();
            }
            (true, false) => {
                //println!("failed to optimize: {} to {}", size_old, size_new);
            }
            (false, true) => {
                // image was altered, BAD; don't overwrite, dorollback
                println!("image altered! :(");
                std::fs::copy(backup_image, self.path).unwrap();
            }
            (false, false) => {
                // wtf!
                panic!();
            }
        }
    }
    fn optimize(&mut self) {
        let original_size = std::fs::metadata(&self.path).unwrap().len();
        let mut iteration = 0;

        let tmp_path = {
            let mut t = self.path.clone();
            t.set_file_name(format!(
                "{}_tmp.png",
                &self.path.file_stem().unwrap().to_str().unwrap()
            ));
            t
        };

        let mut size_before = original_size;
        let mut size_after = 0;
        let mut perc_delta: f64 = 0.0;
        let mut size_delta: i64 = 0;
        while size_before > size_after || iteration == 0 {
            iteration += 1;
            size_before = std::fs::metadata(&self.path).unwrap().len();

            self.run_imagemagick(&tmp_path);
            self.verify_image(&tmp_path);

            self.run_optipng(&tmp_path);
            self.verify_image(&tmp_path);

            self.run_advpng(&tmp_path);
            self.verify_image(&tmp_path);

            self.run_oxipng(&tmp_path);
            self.verify_image(&tmp_path);

            size_after = std::fs::metadata(&self.path).unwrap().len();
            size_delta = size_after as i64 - size_before as i64;
            perc_delta = (size_delta as f64 / size_before as f64) * 100_f64;
        }
        if tmp_path.exists() {
            // clean up
            std::fs::remove_file(tmp_path).unwrap();
        }

        println!(
            "optimized {}, from {}b to {}b, {}, {}",
            self.path.display(),
            original_size,
            size_after,
            size_delta,
            {
                let mut t: String = format!("{}", perc_delta);
                if t.len() > 4 {
                    t = t[0..3].to_string();
                };
                t
            },
        )
    }
}
