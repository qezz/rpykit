use std::{path::PathBuf, time::Instant};

use clap::{Parser, Subcommand};

use rpykit::archive::{extract::ExtractMode, guess_number_of_threads, parallel, sequential};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

// TODO: Implement subcommands:
//   list
//   debug-inspect
#[derive(Subcommand)]
enum Command {
    /// Extract RPA archives
    #[command(aliases = ["x"])]
    Extract {
        /// Output directory. Default: "." (current directory)
        #[arg(short, long, short_aliases = ['C'])]
        output_dir: Option<PathBuf>,

        /// Use explicit number of threads for multithreading. Default: (nproc / 2)
        #[arg(long, short = 'j')]
        num_threads: Option<usize>,

        files: Vec<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Extract {
            output_dir,
            num_threads,
            files,
        } => {
            let output_dir = output_dir.unwrap_or_else(|| PathBuf::from("."));
            let num_threads = num_threads.unwrap_or_else(|| guess_number_of_threads() / 2);
            let mode = if num_threads == 1 {
                ExtractMode::Sequential
            } else {
                ExtractMode::Parallel(num_threads)
            };

            let start = Instant::now();
            let res = match mode {
                ExtractMode::Parallel(n) => parallel::extract_all(&files, output_dir, n),
                ExtractMode::Sequential => sequential::extract_all(&files, output_dir),
            };
            let took = start.elapsed();

            match res {
                Ok(_) => {
                    println!("Finished: threads: {}, took: {:?}", num_threads, took);
                }
                Err(e) => {
                    println!("Error: {:#?}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
