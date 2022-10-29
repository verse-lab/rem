extern crate regex;

use std::borrow::Cow;
use std::process::Command;
use std::vec::IntoIter;
use regex::Regex;

pub fn compile_file(file_name: &str, args: &Vec<&str>) -> Command {
    let mut compile = Command::new("rustc");
    for arg in args {
        compile
            .arg(arg);
    }
    compile.arg(file_name);
    compile
}

pub fn lift_general_help<'a>(stderr: &'a Cow<str>) -> IntoIter<(usize, &'a str)> {
    let lines = stderr.split("\n");
    let mut help_lines: Vec<(usize, &str)> = Vec::new();
    let mut check_for_help = false;
    let mut lines_it = lines.enumerate();
    loop {
        let line = match lines_it.next() {
            Some(line) => line,
            None => break,
        };

        if check_for_help {
            check_for_help = false;
            let line_split = line.1.split(" | ");
            let mut it = line_split.enumerate();
            let line_number = match it.next() {
                Some((_, line_number)) => match line_number.parse::<usize>() {
                    Ok(line_number) => line_number,
                    Err(_) => continue,
                },
                None => continue,
            };
            let line_text = match it.next() {
                Some((_, line_text)) => line_text,
                None => continue,
            };
            help_lines.push((line_number, line_text));
        }

        if line.1.starts_with("help: consider") {
            lines_it.next(); // dump empty line
            check_for_help = true;
        }
    }
    help_lines.into_iter()
}

pub fn lift_lifetime_constraint<'a>(stderr: &'a Cow<str>) -> IntoIter<(usize, &'a str)> {
    let lines = stderr.split("\n");
    let mut help_lines: Vec<(usize, &str)> = Vec::new();

    println!("getting lifetime constraints...");
    let re = Regex::new(r"(?s).*(?-s)\n(?P<line_number>\d+) \| (?P<fn_sig>fn .+) \{?(?s).*(?-s)= help: consider.+bound: `(?P<constraint_lhs>'[a-z]+): (?P<constraint_rhs>'[a-z]+)`").unwrap();
    for caps in re.captures_iter(stderr) {
        println!(
            "line: {:?}, fn: {:?}, {:?}:{:?}",
            &caps["line_number"],
            &caps["fn_sig"],
            &caps["constraint_lhs"],
            &caps["constraint_rhs"],
        )
    }
    vec![].into_iter()
}

pub fn repair_iteration(compile_cmd: &mut Command, process_errors: &dyn Fn(&Cow<str>) -> bool) -> bool {
    loop {
        let out = compile_cmd.output().unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stderr.len() == 0 {
            return true;
        }

        if !process_errors(&stderr) {
            return false;
        }
    }
}