extern crate serde;
extern crate serde_json;

use std::borrow::Cow;
use std::fmt::format;
use std::fs;
use std::vec::IntoIter;
use regex::Regex;
use rustfix::diagnostics::Diagnostic;

use serde_json::{Deserializer, Value};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct ErrorSpanText {
    text: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ErrorSpan {
    file_name: String,
    line_start: i32,
    line_end: i32,
    is_primary: bool,
    text: Vec<ErrorSpanText>,
}

#[derive(Serialize, Deserialize, Debug)]
struct HelpMessage {
    spans: Vec<ErrorSpan>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CompilerError {
    children: Vec<HelpMessage>,
    rendered: String,
}

use crate::repair_system::RepairSystem;
use crate::common;

pub struct Repairer {}

fn repair_standard_help(stderr: &Cow<str>, new_file_name: &str) -> bool {
    let binding = stderr.to_string();
    let deserializer = serde_json::Deserializer::from_str(binding.as_str());
    let stream = deserializer.into_iter::<CompilerError>();
    let mut helped = false;
    for item in stream {
        let rendered = item.unwrap().rendered;

        let re = Regex::new(r"(?s)(?P<replacer>.*)(?-s)help: consider.+\n.*\n(?P<line_number>\d+) \| (?P<replacement>.+)\n").unwrap();
        let mut help_lines = re.captures_iter(rendered.as_str());

        for captured in help_lines {
            // println!("{}, {}\n{}", &captured["line_number"], &captured["replacement"], &captured["replacer"]);
            let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();
            let line_number = match captured["line_number"].parse::<usize>() {
                Ok(n) => n,
                Err(_) => continue,
            };
            let find_line_re_str = format!("{} \\| (?P<to_replace>.+)\n", line_number);
            let find_line_re = Regex::new(find_line_re_str.as_str()).unwrap();
            let mut captures_line = find_line_re.captures_iter(&captured["replacer"]);
            match captures_line.next() {
                Some(captured_line) => {
                    helped = true;
                    let replace_re = Regex::new(regex::escape(&captured_line["to_replace"]).as_str()).unwrap();
                    let new_line = format!("{}", &captured["replacement"]);
                    let new_file_content = replace_re.replace_all(file_content.as_str(), new_line.as_str());
                    fs::write(new_file_name.to_string(), new_file_content.to_string()).unwrap();
                },
                None => {},
            }
        }
    }
    helped
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
        let where_re = Regex::new(r"(?P<where>where (?s).*(?-s))\{").unwrap();
        let mut captures_where = where_re.captures_iter(&captured["fn_sig"]);

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
        let args : Vec<&str> = vec!["--error-format=json"];
        fs::copy(file_name, &new_file_name).unwrap();

        let mut compile_cmd = common::compile_file(&new_file_name, &args);


        let process_errors = |stderr: &Cow<str>| {
            repair_bounds_help(stderr, new_file_name) ||
            repair_standard_help(stderr, new_file_name)
        };

        common::repair_iteration(&mut compile_cmd, &process_errors, true)
    }
}