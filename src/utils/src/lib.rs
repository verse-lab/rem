#![feature(rustc_private)]
#![feature(box_patterns)]
#![feature(never_type)]
#![feature(fs_try_exists)]
#![feature(is_some_and)]
#![feature(iter_intersperse)]
#![feature(box_syntax)]

extern crate rustc_driver;
pub extern crate rustc_lint;
pub extern crate rustc_span;
pub extern crate string_cache;

pub mod annotation;
pub mod error;
pub mod filesystem;
pub mod formatter;
pub mod labelling;
pub mod location;
pub mod macros;
pub mod parser;
pub mod typ;
pub mod wrappers;

use std::fs;
use log::debug;
use quote::ToTokens;
use std::io::Write;
use std::process::{Command, Stdio};

use syn::visit_mut::VisitMut;
use syn::{ExprCall, ExprMethodCall, File, ImplItemMethod, ItemFn, TraitItemMethod};

////////////////////////////////////////////////////////////////////////////////////////////////////
////////////////////////////////        COMPILE        /////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////////////////////////
pub fn compile_file(file_name: &str, args: &Vec<&str>) -> Command {
    let mut compile = Command::new("rustc");
    for arg in args {
        compile.arg(arg);
    }
    compile.arg(file_name);
    compile
}

pub fn check_project(manifest_path: &str, cargo_args: &Vec<&str>) -> Command {
    let mut check = Command::new("cargo");
    check.arg("check");
    for arg in cargo_args {
        check.arg(arg);
    }
    let toml = format!("--manifest-path={}", manifest_path);
    check.arg(toml);
    check.arg("--message-format=json");
    check
}

pub fn build_project(manifest_path: &str, cargo_args: &Vec<&str>) -> Command {
    let mut check = Command::new("cargo");
    check.arg("build");
    for arg in cargo_args {
        check.arg(arg);
    }
    let toml = format!("--manifest-path={}", manifest_path);
    check.arg(toml);
    check.arg("--message-format=json");
    check
}

pub struct FindCallee<'a> {
    pub found: bool,
    pub callee_fn_name: &'a str,
}

impl VisitMut for FindCallee<'_> {
    fn visit_expr_call_mut(&mut self, i: &mut ExprCall) {
        let callee = i.func.as_ref().into_token_stream().to_string();
        debug!("looking at callee: {}", callee);
        match callee.contains(self.callee_fn_name) {
            true => self.found = true,
            false => syn::visit_mut::visit_expr_call_mut(self, i),
        }
    }

    fn visit_expr_method_call_mut(&mut self, i: &mut ExprMethodCall) {
        let callee = i.method.clone().into_token_stream().to_string();
        debug!("looking at callee: {}", callee);
        match callee.contains(self.callee_fn_name) {
            true => self.found = true,
            false => syn::visit_mut::visit_expr_method_call_mut(self, i),
        }
    }
}

pub struct FindCaller<'a> {
    caller_fn_name: &'a str,
    callee_finder: &'a mut FindCallee<'a>,
    found: bool,
    caller: String,
}

impl VisitMut for FindCaller<'_> {
    fn visit_impl_item_method_mut(&mut self, i: &mut ImplItemMethod) {
        if self.found {
            return;
        }
        debug!("{:?}", i);
        let id = i.sig.ident.to_string();
        match id == self.caller_fn_name {
            true => {
                self.callee_finder.visit_impl_item_method_mut(i);
                if !self.callee_finder.found {
                    return;
                }
                self.found = true;
                self.caller = i.into_token_stream().to_string();
            }
            false => {}
        }
        syn::visit_mut::visit_impl_item_method_mut(self, i);
    }

    fn visit_trait_item_method_mut(&mut self, i: &mut TraitItemMethod) {
        if self.found {
            return;
        }
        debug!("{:?}", i);
        let id = i.sig.ident.to_string();
        match id == self.caller_fn_name {
            true => {
                self.callee_finder.visit_trait_item_method_mut(i);
                if !self.callee_finder.found {
                    return;
                }
                self.found = true;
                self.caller = i.into_token_stream().to_string();
            }
            false => {}
        }
        syn::visit_mut::visit_trait_item_method_mut(self, i);
    }

    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        if self.found {
            return;
        }
        debug!("{:?}", i);
        let id = i.sig.ident.to_string();
        match id == self.caller_fn_name {
            true => {
                self.callee_finder.visit_item_fn_mut(i);
                if !self.callee_finder.found {
                    return;
                }
                self.found = true;
                self.caller = i.into_token_stream().to_string();
            }
            false => (),
        }
    }
}

pub struct FindFn<'a> {
    fn_name: &'a str,
    found: bool,
    fn_txt: String,
}

impl VisitMut for FindFn<'_> {
    fn visit_impl_item_method_mut(&mut self, i: &mut ImplItemMethod) {
        if self.found {
            return;
        }
        debug!("{:?}", i);
        let id = i.sig.ident.to_string();
        match id == self.fn_name {
            true => {
                self.found = true;
                self.fn_txt = i.into_token_stream().to_string();
            }
            false => {}
        }
        syn::visit_mut::visit_impl_item_method_mut(self, i);
    }

    fn visit_trait_item_method_mut(&mut self, i: &mut TraitItemMethod) {
        if self.found {
            return;
        }
        debug!("{:?}", i);
        let id = i.sig.ident.to_string();
        match id == self.fn_name {
            true => {
                self.found = true;
                self.fn_txt = i.into_token_stream().to_string();
            }
            false => {}
        }
        syn::visit_mut::visit_trait_item_method_mut(self, i);
    }

    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        if self.found {
            return;
        }
        debug!("{:?}", i);
        let id = i.sig.ident.to_string();
        match id == self.fn_name {
            true => {
                self.found = true;
                self.fn_txt = i.into_token_stream().to_string();
            }
            false => (),
        }
    }
}

pub fn find_caller(file_name: &str, caller_name: &str, callee_name: &str) -> (bool, String, String) {
    let file_content: String = fs::read_to_string(&file_name).unwrap().parse().unwrap();
    let mut file = syn::parse_str::<File>(file_content.as_str())
        .map_err(|e| format!("{:?}", e))
        .unwrap();

    let mut visit = FindCaller { caller_fn_name:caller_name, callee_finder: &mut FindCallee { found: false, callee_fn_name: callee_name}, found: false, caller: String::new()};
    visit.visit_file_mut(&mut file);

    let mut callee = FindFn {
        fn_name: callee_name,
        found: false,
        fn_txt: String::new(),
    };

    callee.visit_file_mut(&mut file);

    (visit.found && callee.found, format_source(visit.caller.as_str()), format_source(callee.fn_txt.as_str()))
}

////////////////////////////////////////////////////////////////////////////////////////////////////
////////////////////////////////          MISC          ////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////////////////////////

pub fn format_source(src: &str) -> String {
    let rustfmt = {
        let mut proc = Command::new(&"rustfmt")
            .arg("--edition=2021")
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
