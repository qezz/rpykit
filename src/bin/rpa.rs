use std::{io::Write, path::PathBuf, time::Instant};

use clap::{Parser, Subcommand};

use rpykit::archive::{
    Archive, extract::ExtractMode, guess_number_of_threads, parallel, sequential,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List files in RPA archives without extracting them
    #[command(aliases = ["t", "list"])]
    Inspect {
        /// Show file sizes in bytes alongside names
        #[arg(short, long)]
        verbose: bool,

        #[arg(required = true)]
        files: Vec<PathBuf>,
    },

    /// Extract RPA archives
    #[command(aliases = ["x"])]
    Extract {
        /// Output directory. Default: "." (current directory)
        #[arg(short, long, short_aliases = ['C'])]
        output_dir: Option<PathBuf>,

        /// Use explicit number of threads for multithreading. Default: (nproc / 2)
        #[arg(long, short = 'j')]
        num_threads: Option<usize>,

        #[arg(required = true)]
        files: Vec<PathBuf>,
    },
}

fn inspect(files: &[PathBuf], verbose: bool) -> Result<(), String> {
    let mut output = std::io::stdout().lock();

    for path in files {
        let archive =
            Archive::read_from_file(path).map_err(|err| format!("{}: {err:?}", path.display()))?;

        let mut entries: Vec<_> = archive.index.iter().collect();
        entries.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));

        for (name, entry) in entries {
            let result = if verbose {
                writeln!(output, "{:>12} {}", entry.len, name)
            } else {
                writeln!(output, "{}", name)
            };

            result.map_err(|err| format!("Failed to write listing: {err}"))?;
        }
    }

    output
        .flush()
        .map_err(|err| format!("Failed to write listing: {err}"))
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Inspect { verbose, files } => {
            if let Err(err) = inspect(&files, verbose) {
                eprintln!("Error: {err}");
                std::process::exit(1);
            }
        }
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
