extern crate regex;
extern crate serde;
extern crate radix_fmt;

use std::borrow::Cow;
use std::fs;
use std::io::{BufWriter, Write};
use std::process::{Command, Stdio};
use proc_macro2::{Span};
use quote::ToTokens;
use syn::{FnArg, Lifetime, LifetimeDef, PredicateLifetime, Type, TypeReference, visit_mut::VisitMut, WhereClause, WherePredicate};
use regex::{Regex, escape};
use serde::{Serialize, Deserialize};

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

        let out_file = fs::File::create(&new_file_name).unwrap();
        let mut writer = BufWriter::new(out_file);
        for captured in help_lines {
            println!(
                "line: {:?}, fn: {:?} {}",
                &captured["line_number"],
                &captured["replacement"],
                current_line,
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
    success: bool
}

impl VisitMut for FnLifetimeBounder<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut syn::ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.fn_name.to_string() {
            false => (),
            true => {
                let gen = &mut i.sig.generics;
                let wc = gen.where_clause.get_or_insert(WhereClause{ where_token: Default::default(), predicates: Default::default() });
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

pub fn repair_bounds_help(stderr: &Cow<str>, new_file_name: &str, fn_name : &str) -> bool {
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
            let mut file = syn::parse_str::<syn::File>(file_content.as_str()).map_err(|e| format!("{:?}", e)).unwrap();
            let mut visit = FnLifetimeBounder { fn_name, lifetime: &captured["constraint_lhs"], bound: &captured["constraint_rhs"], success: false };
            visit.visit_file_mut(&mut file);
            let file = file.into_token_stream().to_string();
            match visit.success {
                true => {
                    fs::write(new_file_name.to_string(), format_source(&file)).unwrap();
                    helped = true;
                },
                false => ()
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
            .spawn().unwrap();
        let mut stdin = proc.stdin.take().unwrap();
        stdin.write_all(src.as_bytes()).unwrap();
        proc
    };

    let stdout = rustfmt.wait_with_output().unwrap();

    String::from_utf8(stdout.stdout).unwrap()
}

// Tight Lifetime Annotator
struct TightLifetimeAnnotatorTypeHelper {}

impl VisitMut for TightLifetimeAnnotatorTypeHelper {
    fn visit_type_mut(&mut self, i: &mut Type) {
        match i {
            Type::Reference(r) =>
                {
                    r.lifetime = Some(Lifetime::new("'lt0", Span::call_site()));
                    self.visit_type_mut(r.elem.as_mut());
                },
            _ => ()
        }
    }
}

struct TightLifetimeAnnotatorFnArgHelper {}

impl VisitMut for TightLifetimeAnnotatorFnArgHelper {
    fn visit_fn_arg_mut(&mut self, i: &mut FnArg) {
        match i {
            FnArg::Receiver(_) => (), // don't modify receiver yet (&self)
            FnArg::Typed(t) => {
                let mut type_helper = TightLifetimeAnnotatorTypeHelper {};
                type_helper.visit_type_mut(t.ty.as_mut())
            },
        }
    }
}

struct TightLifetimeAnnotator<'a> {
    fn_name : &'a str,
    success : bool
}

impl VisitMut for TightLifetimeAnnotator<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut syn::ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.fn_name.to_string() {
            false => (),
            true =>
                match (&mut i.sig.inputs, &mut i.sig.generics, &mut i.sig.output)  {
                    (inputs, _, _) if inputs.len() == 0 => self.success = true,
                    (_, gen, _) if gen.params.iter().any(|x| match x {
                        syn::GenericParam::Lifetime(_) => true,
                        _ => false
                    })=> self.success = false,
                    (inputs, gen, out) =>
                        {
                            let lifetime = Lifetime::new("'lt0", Span::call_site());
                            gen.params.push(syn::GenericParam::Lifetime(LifetimeDef { attrs: vec![], lifetime, colon_token: None, bounds: Default::default() }));
                            inputs.iter_mut().map(|arg| {
                                let mut fn_arg_helper = TightLifetimeAnnotatorFnArgHelper {};
                                fn_arg_helper.visit_fn_arg_mut(arg)
                            }).all(|_| true);
                            match out {
                                syn::ReturnType::Type(_, ty) => {
                                    match ty.as_mut() {
                                        syn::Type::Reference(r) =>
                                            {
                                                r.lifetime = Some(Lifetime::new("'lt0", Span::call_site()))
                                            },
                                        _ => ()
                                    }
                                },
                                _ => ()
                            };
                            self.success = true
                        }
                }
        }
    }
}

pub fn annotate_tight_named_lifetime(new_file_name: &str, fn_name: &str) -> bool {
    let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();
    let mut file = syn::parse_str::<syn::File>(file_content.as_str()).map_err(|e| format!("{:?}", e)).unwrap();
    let mut visit = TightLifetimeAnnotator { fn_name, success: false };
    visit.visit_file_mut(&mut file);
    let file = quote::ToTokens::into_token_stream(file).to_string();
    match visit.success {
        true => {
            fs::write(new_file_name.to_string(), format_source(&file)).unwrap();
            true
        },
        false => false
    }

}

// Loose Lifetime Annotator
struct LooseLifetimeAnnotatorTypeHelper {
    lt_num : i32,
}

impl VisitMut for LooseLifetimeAnnotatorTypeHelper {
    fn visit_type_mut(&mut self, i: &mut Type) {
        match i {
            Type::Reference(r) =>
                {
                    r.lifetime = Some(Lifetime::new(format!("'lt{}", self.lt_num).as_str(), Span::call_site()));
                    self.lt_num += 1;
                    self.visit_type_mut(r.elem.as_mut());
                },
            _ => ()
        }
    }
}

struct LooseLifetimeAnnotatorFnArgHelper {
    lt_num : i32
}

impl VisitMut for LooseLifetimeAnnotatorFnArgHelper {
    fn visit_fn_arg_mut(&mut self, i: &mut FnArg) {
        match i {
            FnArg::Receiver(_) => (), // don't modify receiver yet (&self)
            FnArg::Typed(t) => {
                let mut type_helper = LooseLifetimeAnnotatorTypeHelper { lt_num: self.lt_num };
                type_helper.visit_type_mut(t.ty.as_mut());
                self.lt_num = type_helper.lt_num
            },
        }
    }
}

struct LooseLifetimeAnnotator<'a> {
    fn_name : &'a str,
    lt_num : i32,
    success : bool
}

impl VisitMut for LooseLifetimeAnnotator<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut syn::ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.fn_name.to_string() {
            false => (),
            true =>
                match (&mut i.sig.inputs, &mut i.sig.generics, &mut i.sig.output)  {
                    (inputs, _, _) if inputs.len() == 0 => self.success = true,
                    (_, gen, _) if gen.params.iter().any(|x| match x {
                        syn::GenericParam::Lifetime(_) => true,
                        _ => false
                    })=> self.success = false,
                    (inputs, gen, out) =>
                        {
                            inputs.iter_mut().map(|arg| {
                                let mut fn_arg_helper = LooseLifetimeAnnotatorFnArgHelper { lt_num: self.lt_num};
                                fn_arg_helper.visit_fn_arg_mut(arg);
                                self.lt_num = fn_arg_helper.lt_num
                            }).all(|_| true);
                            match out {
                                syn::ReturnType::Type(_, ty) => {
                                    match ty.as_mut() {
                                        syn::Type::Reference(r) =>
                                            {
                                                r.lifetime = Some(Lifetime::new(format!("'lt{}", self.lt_num).as_str(), Span::call_site()));
                                                self.lt_num += 1;
                                            },
                                        _ => ()
                                    }
                                },
                                _ => ()
                            };
                            for lt in 0..self.lt_num {
                                let lifetime = Lifetime::new(format!("'lt{}", lt).as_str(), Span::call_site());
                                gen.params.push(syn::GenericParam::Lifetime(LifetimeDef { attrs: vec![], lifetime, colon_token: None, bounds: Default::default() }))
                            }
                            self.success = true
                        }
                }
        }
    }
}

pub fn annotate_loose_named_lifetime(new_file_name: &str, fn_name: &str) -> bool {
    let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();
    let mut file = syn::parse_str::<syn::File>(file_content.as_str()).map_err(|e| format!("{:?}", e)).unwrap();
    let mut visit = LooseLifetimeAnnotator { fn_name, success: false, lt_num: 0 };
    visit.visit_file_mut(&mut file);
    let file = quote::ToTokens::into_token_stream(file).to_string();
    match visit.success {
        true => {
            fs::write(new_file_name.to_string(), format_source(&file)).unwrap();
            true
        },
        false => false
    }

}

struct BoundsLoosener<'a> {
    fn_name : &'a str,
    arg_name : &'a str,
    success : bool
}

struct ArgBoundLoosener<'a> {
    arg_name : &'a str,
    lt : &'a str,
    success: bool
}

impl VisitMut for ArgBoundLoosener<'_> {
    fn visit_fn_arg_mut(&mut self, i: &mut FnArg) {
        match i {
            FnArg::Receiver(_) => (), // don't modify receiver yet (&self)
            FnArg::Typed(t) => {
                match t.pat.as_mut() {
                    syn::Pat::Ident(id) if id.ident.to_string() == self.arg_name => {
                        match t.ty.as_mut() {
                            syn::Type::Reference(r) => {
                                r.lifetime = Some(Lifetime::new(self.lt, Span::call_site()));
                                self.success = true
                            },
                            _ => ()
                        }
                    },
                    _ => ()
                }
            },
        }
    }
}

impl VisitMut for BoundsLoosener<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut syn::ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.fn_name.to_string() {
            false => (),
            true => {
                let mut lt_count = 0;
                let gen = &mut i.sig.generics;
                for i in &gen.params {
                    match i {
                        syn::GenericParam::Lifetime(LifetimeDef { .. }) => lt_count += 1,
                        _ => ()
                    }
                };
                let lt = format!("'lt{}", lt_count);
                let lifetime = Lifetime::new(lt.as_str(), Span::call_site());
                gen.params.push(syn::GenericParam::Lifetime(LifetimeDef { attrs: vec![], lifetime, colon_token: None, bounds: Default::default() }));
                let mut arg_loosener = ArgBoundLoosener { arg_name: self.arg_name, lt: lt.as_str(), success: false };
                let inputs = &mut i.sig.inputs;
                inputs.iter_mut().map(|arg| arg_loosener.visit_fn_arg_mut(arg)).all(|_| true);
                match arg_loosener.success {
                    true => self.success = true,
                    false => ()
                }
            }
        }
    }
}

pub fn loosen_bounds(stderr: &Cow<str>, new_file_name: &str, fn_name: &str) -> bool {
    let binding = stderr.to_string();
    let deserializer = serde_json::Deserializer::from_str(binding.as_str());
    let stream = deserializer.into_iter::<CompilerError>();
    let mut helped = false;
    for item in stream {
        let rendered = item.unwrap().rendered;
        let reference_re = Regex::new(r"error.*`(?P<ref_full>\**(?P<ref>[a-z]+))`").unwrap();
        let error_lines = reference_re.captures_iter(rendered.as_str());

        for captured in error_lines {
            //println!("ref_full: {}, ref: {}", &captured["ref_full"], &captured["ref"]);
            let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();
            let mut file = syn::parse_str::<syn::File>(file_content.as_str()).map_err(|e| format!("{:?}", e)).unwrap();
            let mut visit = BoundsLoosener { fn_name, arg_name: &captured["ref"], success: false };
            visit.visit_file_mut(&mut file);
            let file = quote::ToTokens::into_token_stream(file).to_string();
            match visit.success {
                true => {
                    fs::write(new_file_name.to_string(), format_source(&file)).unwrap();
                    helped = true
                },
                false => ()
            }
        }

    }
    helped
}

pub fn repair_iteration(compile_cmd: &mut Command, process_errors: &dyn Fn(&Cow<str>) -> bool, print_stats: bool, max_iterations: Option<i32>) -> bool {
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