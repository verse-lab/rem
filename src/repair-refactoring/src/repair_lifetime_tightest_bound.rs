use std::borrow::Cow;
use std::fs;

use crate::common::{RepairSystem, compile_file, repair_iteration, repair_standard_help, repair_bounds_help, annotate_named_lifetime, loosen_bounds};

pub struct Repairer {}

impl RepairSystem for Repairer {
    fn name(&self) -> &str {
        "_tightest_bounds_repairer"
    }

    fn repair_file(&self, _: &str, new_file_name: &str) -> bool {
        let args : Vec<&str> = vec!["--error-format=json"];

        let mut compile_cmd = compile_file(&new_file_name, &args);

        let process_errors = |stderr: &Cow<str>| {
            repair_bounds_help(stderr, new_file_name) ||
                repair_standard_help(stderr, new_file_name)
        };

        repair_iteration(&mut compile_cmd, &process_errors, true)
    }

    fn repair_function(&self, file_name: &str, new_file_name: &str, function_sig: &str, function_name: &str) -> bool {
        fs::copy(file_name, &new_file_name).unwrap();
        annotate_named_lifetime(&new_file_name, function_sig);
        // println!("annotated: {}", fs::read_to_string(&new_file_name).unwrap());
        let args : Vec<&str> = vec!["--error-format=json"];

        let mut compile_cmd = compile_file(&new_file_name, &args);

        let process_errors = |stderr: &Cow<str>| {
            let simple_repairs = repair_bounds_help(stderr, new_file_name) ||
                repair_standard_help(stderr, new_file_name);
            if simple_repairs {
                true
            } else {
                loosen_bounds(stderr, new_file_name, function_sig, function_name)
            }
        };

        repair_iteration(&mut compile_cmd, &process_errors, true)
    }
}