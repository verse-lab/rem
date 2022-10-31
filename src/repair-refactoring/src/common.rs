extern crate regex;
extern crate serde;

use std::borrow::Cow;
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::process::Command;
use regex::Regex;

pub trait RepairSystem {
    fn name(&self) -> &str;
    fn repair_file(&self, file_name: &str, new_file_name: &str) -> bool;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CompilerError {
    pub rendered: String,
}

use serde::{Serialize, Deserialize};

pub fn compile_file(file_name: &str, args: &Vec<&str>) -> Command {
    let mut compile = Command::new("rustc");
    for arg in args {
        compile
            .arg(arg);
    }
    compile.arg(file_name);
    compile
}

pub fn repair_standard_help(stderr: &Cow<str>, new_file_name: &str) -> bool {
    let binding = stderr.to_string();
    let deserializer = serde_json::Deserializer::from_str(binding.as_str());
    let stream = deserializer.into_iter::<CompilerError>();
    let mut helped = false;
    for item in stream {
        let rendered = item.unwrap().rendered;
        let re = Regex::new(r"help: consider.+\n.*\n(?P<line_number>\d+) \| (?P<replacement>.+)\n").unwrap();
        let help_lines = re.captures_iter(rendered.as_str());

        let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();

        let lines = file_content.split("\n");
        let mut lines_modifiable = Vec::new();
        for (_, line) in lines.enumerate() {
            lines_modifiable.push(line);
        }

        let mut current_line = 0;

        let out_file = File::create(&new_file_name).unwrap();
        let mut writer = BufWriter::new(out_file);
        for captured in help_lines {
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
                writeln!(writer, "{}", lines_modifiable[current_line]).unwrap();
                current_line += 1;
            }
            writeln!(writer, "{}", replacement).unwrap();
            current_line += 1;
        }
        while current_line < lines_modifiable.len() {
            writeln!(writer, "{}", lines_modifiable[current_line]).unwrap();
            current_line += 1;
        }
    }
    helped
}

pub fn repair_bounds_help(stderr: &Cow<str>, new_file_name: &str) -> bool {
    let binding = stderr.to_string();
    let deserializer = serde_json::Deserializer::from_str(binding.as_str());
    let stream = deserializer.into_iter::<CompilerError>();
    let mut helped = false;
    for item in stream {
        let rendered = item.unwrap().rendered;
        let re = Regex::new(r"(?P<line_number>\d+) \| (?P<fn_sig>fn .+) \{(?s).*(?-s)= help: consider.+bound: `(?P<constraint_lhs>'[a-z]+): (?P<constraint_rhs>'[a-z]+)`").unwrap();
        let help_lines = re.captures_iter(rendered.as_str());
        /*
            &caps["line_number"],
            &caps["fn_sig"],
            &caps["constraint_lhs"],
            &caps["constraint_rhs"],
        */
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

    }
    helped
}

pub fn repair_iteration(compile_cmd: &mut Command, process_errors: &dyn Fn(&Cow<str>) -> bool, print_stats: bool) -> bool {
    let mut count = 0;
    let result = loop {
        let out = compile_cmd.output().unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stderr.len() == 0 {
            break true;
        }
        count += 1;
        if !process_errors(&stderr) {
            break false;
        }

    };

    if print_stats {
        println!("repair count: {}", count);
    }

    result
}