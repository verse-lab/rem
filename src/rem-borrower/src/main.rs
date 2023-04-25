mod borrow;

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::time::SystemTime;
use std::{env, fs};
use rem_utils::compile_file;

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
        mut_method_call_expr_file: String,
        caller_fn_name: String,
        callee_fn_name: String,
        pre_extract_file_name: String,
    },
    /// Test the borrower on inputs
    Test {},
}

fn main() {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();
    let args = Cli::parse();
    match &args.command {
        Commands::Test {} => test(),
        Commands::Run {
            file_name,
            new_file_name,
            mut_method_call_expr_file,
            caller_fn_name,
            callee_fn_name,
            pre_extract_file_name,
        } => {
            let _ = borrow::make_borrows(
                file_name.as_str(),
                new_file_name.as_str(),
                mut_method_call_expr_file.as_str(),
                callee_fn_name.as_str(),
                caller_fn_name.as_str(),
                pre_extract_file_name.as_str(),
            );
        }
    }
}

fn test() {
    for file in fs::read_dir("./input").unwrap() {
        let test_name = file.unwrap().file_name().to_owned();
        if test_name.to_str().unwrap() == "borrow.rs" {
            continue;
        }
        let file_name = format!("./input/{}", test_name.to_str().unwrap());
        let new_file_name = format!("./output/{}", test_name.to_str().unwrap());
        let mut_method_call_expr_file =
            format!("./method_call_mut/{}", test_name.to_str().unwrap());
        let pre_extract_file_name = format!("./pre_extract/{}", test_name.to_str().unwrap());
        let callee_fn_name = "bar";
        let caller_fn_name = "new_foo";
        let now = SystemTime::now();
        borrow::make_borrows(
            file_name.as_str(),
            new_file_name.as_str(),
            mut_method_call_expr_file.as_str(),
            callee_fn_name,
            caller_fn_name,
            pre_extract_file_name.as_str(),
        );
        let time_elapsed = now.elapsed().unwrap();
        let args = vec![];
        let mut compile_cmd = compile_file(new_file_name.as_str(), &args);
        let out = compile_cmd.output().unwrap();
        println!(
            "{}: {} in {:#?}",
            (if out.status.success() {
                format!("PASSED").green()
            } else {
                format!("FAILED").red()
            }),
            test_name.to_str().unwrap(),
            time_elapsed
        );
        println!("------------------------------------------------------------------\n");
    }
}
