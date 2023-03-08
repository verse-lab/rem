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

use log::debug;
use quote::ToTokens;
use std::io::Write;
use std::process::{Command, Stdio};
use syn::visit_mut::VisitMut;
use syn::{ExprCall, ExprMethodCall};

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
