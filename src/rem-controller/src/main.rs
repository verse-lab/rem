mod non_local_controller;
use clap::{Parser, Subcommand};
use colored::Colorize;
use rem_utils::compile_file;
use std::process::exit;
use std::time::SystemTime;
use std::{env, fs};

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
            caller_fn_name,
            callee_fn_name,
        } => {
            if non_local_controller::make_controls(
                file_name.as_str(),
                new_file_name.as_str(),
                callee_fn_name.as_str(),
                caller_fn_name.as_str(),
            ) {
                exit(0)
            } else {
                exit(1)
            }
        }
    }
}

fn test() {
    for file in fs::read_dir("./input").unwrap() {
        let test_name = file.unwrap().file_name().to_owned();
        if test_name.to_str().unwrap() == "if_return_unit_controller.rs" {
            continue;
        }
        if !test_name.to_str().unwrap().contains("qmark_test") {
            continue;
        }
        let file_name = format!("./input/{}", test_name.to_str().unwrap());
        let new_file_name = format!("./output/{}", test_name.to_str().unwrap());
        let callee_fn_name = "bar";
        let caller_fn_name = "new_foo";
        let now = SystemTime::now();
        let success = non_local_controller::make_controls(
            file_name.as_str(),
            new_file_name.as_str(),
            callee_fn_name,
            caller_fn_name,
        );
        let time_elapsed = now.elapsed().unwrap();
        let args = vec![];
        let mut compile_cmd = compile_file(new_file_name.as_str(), &args);
        let out = compile_cmd.output().unwrap();
        println!(
            "{}: {} in {:#?}",
            (if out.status.success() && success {
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
