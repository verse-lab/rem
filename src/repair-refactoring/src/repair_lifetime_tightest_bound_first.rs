use std::borrow::Cow;
use std::fs;

use crate::common::{
    annotate_tight_named_lifetime, compile_file, loosen_bounds, repair_bounds_help,
    repair_iteration, RepairSystem,
};
use crate::repair_lifetime_simple;

pub struct Repairer {}

impl RepairSystem for Repairer {
    fn name(&self) -> &str {
        "_tightest_bounds_first_repairer"
    }

    fn repair_file(&self, file_name: &str, new_file_name: &str) -> bool {
        repair_lifetime_simple::Repairer {}.repair_file(file_name, new_file_name)
    }

    fn repair_function(&self, file_name: &str, new_file_name: &str, fn_name: &str) -> bool {
        fs::copy(file_name, &new_file_name).unwrap();
        annotate_tight_named_lifetime(&new_file_name, fn_name);
        //println!("annotated: {}", fs::read_to_string(&new_file_name).unwrap());
        let args: Vec<&str> = vec!["--error-format=json"];

        let mut compile_cmd = compile_file(&new_file_name, &args);

        let process_errors = |stderr: &Cow<str>| {
            if repair_bounds_help(stderr, new_file_name, fn_name) {
                true
            } else {
                loosen_bounds(stderr, new_file_name, fn_name)
            }
        };

        repair_iteration(&mut compile_cmd, &process_errors, true, Some(10))
    }
}
