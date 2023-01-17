use proc_macro2::Span;
use quote::ToTokens;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fs;
use std::io::{BufWriter, Write};
use std::process::{Command, Stdio};
use syn::{visit_mut::VisitMut, Lifetime, PredicateLifetime, WhereClause, WherePredicate};

pub trait RepairSystem {
    fn name(&self) -> &str;
    fn repair_file(&self, file_name: &str, new_file_name: &str) -> bool;
    fn repair_function(&self, file_name: &str, new_file_name: &str, fn_name: &str) -> bool;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CompilerError {
    pub rendered: String,
}

pub fn compile_file(file_name: &str, args: &Vec<&str>) -> Command {
    let mut compile = Command::new("rustc");
    for arg in args {
        compile.arg(arg);
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
        let re = Regex::new(r"help: consider.+\n.*\n(?P<line_number>\d+) \| (?P<replacement>.+)\n")
            .unwrap();
        let help_lines = re.captures_iter(rendered.as_str());

        let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();

        let lines = file_content.split("\n");
        let mut lines_modifiable = Vec::new();
        for (_, line) in lines.enumerate() {
            lines_modifiable.push(line);
        }

        let mut current_line = 0;

        let out_file = fs::File::create(&new_file_name).unwrap();
        let mut writer = BufWriter::new(out_file);
        for captured in help_lines {
            println!(
                "line: {:?}, fn: {:?} {}",
                &captured["line_number"], &captured["replacement"], current_line,
            );

            let line_number = match captured["line_number"].parse::<usize>() {
                Ok(n) => n,
                Err(_) => continue,
            };
            let replacement = &captured["replacement"];
            if replacement.contains("&'lifetime") {
                continue;
            }

            helped = true;
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

struct FnLifetimeBounder<'a> {
    fn_name: &'a str,
    lifetime: &'a str,
    bound: &'a str,
    success: bool,
}

impl VisitMut for FnLifetimeBounder<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut syn::ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.fn_name.to_string() {
            false => (),
            true => {
                let gen = &mut i.sig.generics;
                let wc = gen.where_clause.get_or_insert(WhereClause {
                    where_token: Default::default(),
                    predicates: Default::default(),
                });
                let mut wp = PredicateLifetime {
                    lifetime: Lifetime::new(self.lifetime, Span::call_site()),
                    colon_token: Default::default(),
                    bounds: Default::default(),
                };
                wp.bounds.push(Lifetime::new(self.bound, Span::call_site()));
                wc.predicates.push(WherePredicate::Lifetime(wp));
                self.success = true
            }
        }
    }
}

pub fn repair_bounds_help(stderr: &Cow<str>, new_file_name: &str, fn_name: &str) -> bool {
    let binding = stderr.to_string();
    let deserializer = serde_json::Deserializer::from_str(binding.as_str());
    let stream = deserializer.into_iter::<CompilerError>();
    let mut helped = false;
    for item in stream {
        let rendered = item.unwrap().rendered;
        let re = Regex::new(r"= help: consider.+bound: `(?P<constraint_lhs>'[a-z0-9]+): (?P<constraint_rhs>'[a-z0-9]+)`").unwrap();
        let help_lines = re.captures_iter(rendered.as_str());
        /*
            &caps["line_number"],
            &caps["fn_sig"],
            &caps["constraint_lhs"],
            &caps["constraint_rhs"],
        */
        for captured in help_lines {
            let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();
            let mut file = syn::parse_str::<syn::File>(file_content.as_str())
                .map_err(|e| format!("{:?}", e))
                .unwrap();
            let mut visit = FnLifetimeBounder {
                fn_name,
                lifetime: &captured["constraint_lhs"],
                bound: &captured["constraint_rhs"],
                success: false,
            };
            visit.visit_file_mut(&mut file);
            let file = file.into_token_stream().to_string();
            match visit.success {
                true => {
                    fs::write(new_file_name.to_string(), format_source(&file)).unwrap();
                    helped = true;
                }
                false => (),
            }
        }
    }
    helped
}

pub fn format_source(src: &str) -> String {
    let rustfmt = {
        let mut proc = Command::new(&"rustfmt")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = proc.stdin.take().unwrap();
        stdin.write_all(src.as_bytes()).unwrap();
        proc
    };

    let stdout = rustfmt.wait_with_output().unwrap();

    String::from_utf8(stdout.stdout).unwrap()
}

pub fn repair_iteration(
    compile_cmd: &mut Command,
    process_errors: &dyn Fn(&Cow<str>) -> bool,
    print_stats: bool,
    max_iterations: Option<i32>,
) -> bool {
    let mut count = 0;
    let max_iterations = max_iterations.unwrap_or(25);
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
        if max_iterations == count {
            break false;
        }
    };

    if print_stats {
        println!("repair count: {}", count);
    }

    result
}
