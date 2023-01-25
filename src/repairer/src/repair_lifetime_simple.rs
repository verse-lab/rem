use std::borrow::Cow;
use std::fs;

use crate::common::{
    compile_file, repair_bounds_help, repair_iteration, repair_standard_help, RepairSystem,
};

pub struct Repairer {}

impl RepairSystem for Repairer {
    fn name(&self) -> &str {
        "_simple_repairer"
    }

    fn repair_file(&self, file_name: &str, new_file_name: &str) -> bool {
        self.repair_function(file_name, new_file_name, "")
    }

    fn repair_function(&self, file_name: &str, new_file_name: &str, fn_name: &str) -> bool {
        let args: Vec<&str> = vec!["--error-format=json"];
        fs::copy(file_name, &new_file_name).unwrap();

        let mut compile_cmd = compile_file(&new_file_name, &args);

        let process_errors = |stderr: &Cow<str>| {
            repair_bounds_help(stderr, new_file_name, fn_name)
                || repair_standard_help(stderr, new_file_name)
        };

        repair_iteration(&mut compile_cmd, &process_errors, true, None)
    }
}