#![feature(rustc_private)]
#![feature(box_patterns)]
#![feature(never_type)]
#![feature(fs_try_exists)]
#![feature(is_some_and)]
#![feature(iter_intersperse)]
#![feature(box_syntax)]

pub extern crate string_cache;
extern crate rustc_driver;
pub extern crate rustc_lint;
pub extern crate rustc_span;

pub mod parser;
pub mod typ;
pub mod labelling;
pub mod location;
pub mod filesystem;
pub mod annotation;
pub mod formatter;
pub mod error;
pub mod macros;
pub mod wrappers;

use std::io::Write;
use std::process::{Command, Stdio};
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

pub fn compile_project(manifest_path: &str, cargo_args: &Vec<&str>) -> Command {
    let mut compile = Command::new("cargo");
    compile.arg("build");
    for arg in cargo_args {
        compile.arg(arg);
    }
    let toml = format!("--manifest-path={}", manifest_path);
    compile.arg(toml);
    compile.arg("--message-format=json");
    compile
}

////////////////////////////////////////////////////////////////////////////////////////////////////
////////////////////////////////          MISC          ////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////////////////////////

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
