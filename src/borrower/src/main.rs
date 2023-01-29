mod borrow;

use std::fs;
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the repairs
    Run {
        file_name: String,
        new_file_name: String,
        caller_fn_name: String,
        callee_fn_name: String,
    },
    /// Test the borrower on inputs
    Test {},
}

fn main() {
    let args = Cli::parse();
    match &args.command {
        Commands::Test {} => test(),
        Commands::Run {
            file_name,
            new_file_name,
            caller_fn_name,
            callee_fn_name,
        } => borrow::make_borrows(
            file_name.as_str(),
            new_file_name.as_str(),
            callee_fn_name.as_str(),
            caller_fn_name.as_str(),
        ),
    }
}

fn test() {
    for file in fs::read_dir("./input").unwrap() {
        println!("{}", file.unwrap().file_name().to_str().unwrap());

    }
}
