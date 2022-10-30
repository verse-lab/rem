use std::borrow::Cow;
use std::fmt::format;
use std::fs;
use std::vec::IntoIter;
use regex::Regex;

use crate::repair_system::RepairSystem;
use crate::common;

pub struct Repairer {}

fn repair_standard_help(stderr: &Cow<str>, new_file_name: &str) -> bool {
    let re = Regex::new(r"help: consider.+\n.*\n(?P<line_number>\d+) \| (?P<replacement>.+)\n").unwrap();
    let mut help_lines = re.captures_iter(stderr);

    let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();

    let lines = file_content.split("\n");
    let mut lines_modifiable = Vec::new();
    for (_, line) in lines.enumerate() {
        lines_modifiable.push(line);
    }

    let mut helped = false;
    let mut current_line = 0;
    let mut content = String::new();

    loop {
        match help_lines.next() {
            Some(captured) => {
                println!(
                    "line: {:?}, fn: {:?} {}",
                    &captured["line_number"],
                    &captured["replacement"],
                    current_line,
                );
                helped = true;
                let line_number = match captured["line_number"].parse::<usize>() {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                let replacement = &captured["replacement"];
                while current_line < line_number - 1 {
                    content.push_str(lines_modifiable[current_line]);
                    content.push('\n');
                    current_line += 1;
                }
                content.push_str(replacement);
                content.push('\n');
                current_line += 1;
            },
            None => {
                while current_line < lines_modifiable.len() {
                    content.push_str(lines_modifiable[current_line]);
                    content.push('\n');
                    current_line += 1;
                }
                fs::write(new_file_name.to_string(), content).unwrap();
                break helped;
            }
        }
    }
}

fn repair_bounds_help(stderr: &Cow<str>, new_file_name: &str) -> bool {
    let re = Regex::new(r"error(?s).*(?-s)(?P<line_number>\d+) \| (?P<fn_sig>fn .+) \{(?s).*(?-s)= help: consider.+bound: `(?P<constraint_lhs>'[a-z]+): (?P<constraint_rhs>'[a-z]+)`(?s).*(?-s)error").unwrap();
    let mut help_lines = re.captures_iter(stderr);
    /*
            &caps["line_number"],
            &caps["fn_sig"],
            &caps["constraint_lhs"],
            &caps["constraint_rhs"],
    */
    let mut helped = false;
    for captured in help_lines {
        helped = true;
        let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();
        let mut where_re_str = format!("{}(?s).*(?-s) (?P<where>where (?s).*(?-s))", regex::escape(&captured["fn_sig"]));
        where_re_str.push_str("\\{");
        let where_re = Regex::new(where_re_str.as_str()).unwrap();
        let mut captures_where = where_re.captures_iter(file_content.as_str());

        match captures_where.next() {
            Some(captured_where) => {
                let replace_re = Regex::new(regex::escape(&captured_where["where"]).as_str()).unwrap();
                let new_where = format!("{}, {}: {}", &captured_where["where"], &captured["constraint_lhs"], &captured["constraint_rhs"]);
                let new_file_content = replace_re.replace_all(file_content.as_str(), regex::escape(new_where.as_str()));
                fs::write(new_file_name.to_string(), new_file_content.to_string()).unwrap();
            },
            None => {
                let replace_re = Regex::new(regex::escape(&captured["fn_sig"]).as_str()).unwrap();
                let new_sig = format!("{} where {}: {}", &captured["fn_sig"], &captured["constraint_lhs"], &captured["constraint_rhs"]);
                let new_file_content = replace_re.replace_all(file_content.as_str(), new_sig.as_str());
                fs::write(new_file_name.to_string(), new_file_content.to_string()).unwrap();
            },
        }
    }
    helped
}

impl RepairSystem for Repairer {
    fn name(&self) -> &str {
        "_simple_repairer"
    }

    fn repair_file(&self, file_name: &str, new_file_name: &str) -> bool {
        let args : Vec<&str> = vec![];
        fs::copy(file_name, &new_file_name).unwrap();

        let mut compile_cmd = common::compile_file(&new_file_name, &args);


        let process_errors = |stderr: &Cow<str>| {
            repair_bounds_help(stderr, new_file_name) ||
            repair_standard_help(stderr, new_file_name)
        };

        common::repair_iteration(&mut compile_cmd, &process_errors, true)
    }
}