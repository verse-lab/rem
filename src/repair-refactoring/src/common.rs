extern crate regex;
extern crate serde;
extern crate radix_fmt;

use std::borrow::Cow;
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::process::Command;
//use radix_fmt::{radix, radix_29};
use regex::{Regex, escape};
use serde::{Serialize, Deserialize};

pub trait RepairSystem {
    fn name(&self) -> &str;
    fn repair_file(&self, file_name: &str, new_file_name: &str) -> bool;
    fn repair_function(&self, file_name: &str, new_file_name: &str, function_sig: &str, function_name: &str) -> bool;
}
/*
fn gen_next_lexico_alpha(current_number: i32) -> String {
    if current_number < 10 {
        let base_29 = radix(current_number + 10, 29).to_string();
        base_29.clone()
    }
    let base_29 = radix(current_number, 29);
    println!("{}", base_29);
    base_29.to_string().clone()
}
*/
#[derive(Serialize, Deserialize, Debug)]
pub struct CompilerError {
    pub rendered: String,
}

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
            let captures_where = where_re.captures(&captured["fn_sig"]);

            match captures_where {
                Some(captured_where) => {
                    let replace_re = Regex::new(escape(&captured_where["where"]).as_str()).unwrap();
                    let new_where = format!("{}, {}: {}", &captured_where["where"], &captured["constraint_lhs"], &captured["constraint_rhs"]);
                    let new_file_content = replace_re.replace_all(file_content.as_str(), escape(new_where.as_str()));
                    fs::write(new_file_name.to_string(), new_file_content.to_string()).unwrap();
                },
                None => {
                    let replace_re = Regex::new(escape(&captured["fn_sig"]).as_str()).unwrap();
                    let new_sig = format!("{} where {}: {}", &captured["fn_sig"], &captured["constraint_lhs"], &captured["constraint_rhs"]);
                    let new_file_content = replace_re.replace_all(file_content.as_str(), new_sig.as_str());
                    fs::write(new_file_name.to_string(), new_file_content.to_string()).unwrap();
                },
            }
        }

    }
    helped
}

// TODO: URGENT: need to rewrite using syn (AST)
pub fn annotate_named_lifetime(new_file_name: &str, function_sig: &str) -> bool {
    let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();
    let re = Regex::new(r"(?P<fn_prefix>.*fn (?P<fn_name>.*))\s?(?P<generic>(<(?P<generic_args>.+)>)?)\((?P<args>.*)\)(?P<ret_ty>.*)?\s?(?P<where>(where)?.*)").unwrap();
    let capture = re.captures(function_sig);

    let success = match capture {
        None => false,
        Some(captured) => {
            match (&captured["where"], &captured["generic"], &captured["args"], &captured["ret_ty"]) {
                ("", "", "", _) => true, // count as success--no annotation needed
                ("", "", args, ret_ty) => {
                    let add_ref_lifetime_re = Regex::new(r"\&").unwrap();
                    let new_args = add_ref_lifetime_re.replace_all(args, r"&'lt0 ");
                    let new_ret_ty = add_ref_lifetime_re.replace_all(ret_ty, r"&'lt0 ");
                    let replace_re = Regex::new(escape(function_sig).as_str()).unwrap();
                    let new_sig = format!("{}<'lt0>({}) {}", &captured["fn_prefix"], new_args, new_ret_ty);
                    let new_file_content = replace_re.replace_all(file_content.as_str(), new_sig.as_str());
                    fs::write(new_file_name.to_string(), new_file_content.to_string()).unwrap();
                    true
                },
                _ => false, // need to support annotating for function already annotated
            }
        },
    };
    success
}

pub fn loosen_bounds(stderr: &Cow<str>, new_file_name: &str, function_sig: &str, function_name: &str) -> bool {
    let binding = stderr.to_string();
    let deserializer = serde_json::Deserializer::from_str(binding.as_str());
    let stream = deserializer.into_iter::<CompilerError>();
    let mut helped = false;
    for item in stream {
        let rendered = item.unwrap().rendered;
        let reference_re = Regex::new(r"error.*`(?P<ref_full>\**(?P<ref>[a-z]+))`").unwrap();
        let error_lines = reference_re.captures_iter(rendered.as_str());

        for captured in error_lines {
            println!("ref_full: {}, ref: {}", &captured["ref_full"], &captured["ref"]);
            let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();
            let mut fn_str_regex = format!(r"(?P<full_fn_sig>.*fn {}\s*(?P<generic>(<(?P<generic_args>.*)>)?)\((?P<args>.*)\)(?P<ret_ty>.*)?\s?(?P<where>(where)?.*))", function_name);
            fn_str_regex.push_str("\\{");
            let re_new_sig = Regex::new(fn_str_regex.as_str()).unwrap();
            let capture_sig = re_new_sig.captures(file_content.as_str()).unwrap();
            let old_full_sig = String::from(&capture_sig["full_fn_sig"]);
            match &capture_sig["generic_args"] {
                "" => {},
                generic_args => {
                    let lifetime_args_re = Regex::new(r"'(?P<named_lifetime>[a-z]+),?").unwrap();
                    let count = lifetime_args_re.captures_iter(&generic_args).count();
                    let new_lt = format!("'lt{}", count);
                    let sig_new_generic_args = format!("<{}, {}>", generic_args, new_lt);
                    let re_gen_replace = Regex::new(format!("<{}>", generic_args).as_str()).unwrap();
                    let new_full_sig = re_gen_replace.replace_all(old_full_sig.as_str(), sig_new_generic_args);
                    match &capture_sig["args"] {
                        "" => {println!("empty args")},
                        args => {
                            let get_ref_arg_re = Regex::new(format!(r"(?P<ref_arg>{}.*:.*(,|\)))", &captured["ref"]).as_str()).unwrap(); // TODO: highly unstable!! need syn
                            match get_ref_arg_re.captures(args) {
                                None => {},
                                Some(captured_ref) => {
                                    println!("captured ref arg: {}", &captured_ref["ref_arg"]);
                                    let replace_ref_re = Regex::new(r"\&(?P<old_lt>'\S*)").unwrap();
                                    let new_ref = replace_ref_re.replace(&captured_ref["ref_arg"], format!("&{}",new_lt));
                                    let get_ref_arg_re = Regex::new(escape(format!("{}", &captured_ref["ref_arg"]).as_str()).as_str()).unwrap();
                                    let new_full_sig = new_full_sig.to_string();
                                    let new_full_sig = get_ref_arg_re.replace_all(new_full_sig.as_str(), new_ref);
                                    let replace_fn_re = Regex::new(escape(old_full_sig.as_str()).as_str()).unwrap();
                                    let new_file_content = replace_fn_re.replace_all(file_content.as_str(), new_full_sig.to_string().as_str());
                                    fs::write(new_file_name.to_string(), new_file_content.to_string()).unwrap();
                                    helped = true;
                                }
                            }

                        },
                    }
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