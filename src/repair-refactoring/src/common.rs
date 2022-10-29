extern crate regex;

use std::borrow::{Borrow, Cow};
use std::iter::Map;
use std::process::Command;
use std::ptr::replace;
use std::vec::IntoIter;
use regex::{CaptureMatches, Captures, Regex};

pub fn compile_file(file_name: &str, args: &Vec<&str>) -> Command {
    let mut compile = Command::new("rustc");
    for arg in args {
        compile
            .arg(arg);
    }
    compile.arg(file_name);
    compile
}

pub fn repair_iteration(compile_cmd: &mut Command, process_errors: &dyn Fn(&Cow<str>) -> bool, print_stats: bool) -> bool {
    let mut count = 0;
    let result = loop {
        count += 1;
        let out = compile_cmd.output().unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stderr.len() == 0 {
            break true;
        }

        if !process_errors(&stderr) {
            break false;
        }
    };

    if print_stats {
        println!("repair count: {}", count);
    }

    result
}