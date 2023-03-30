use crate::common::{repair_iteration, RepairResult, RepairSystem};

use std::collections::HashSet;
use std::fs;
use utils::compile_file;

pub struct Repairer {}

impl RepairSystem for Repairer {
    fn name(&self) -> &str {
        "_rustfix_repairer"
    }

    fn repair_project(
        &self,
        _src_path: &str,
        _manifest_path: &str,
        _fn_name: &str,
    ) -> RepairResult {
        RepairResult {
            success: false,
            repair_count: 0,
            has_non_elidible_lifetime: false,
        }
    }

    fn repair_file(&self, file_name: &str, new_file_name: &str) -> RepairResult {
        fs::copy(file_name, &new_file_name).unwrap();
        let args = vec!["--error-format=json"];

        let mut compile_cmd = compile_file(&new_file_name, &args);

        let process_errors = |stderr: &str| {
            let suggestions = rustfix::get_suggestions_from_json(
                stderr,
                &HashSet::new(),
                rustfix::Filter::Everything,
            )
            .expect("rustfix failed to run on error json");

            if suggestions.len() == 0 {
                return false;
            }

            let code: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();
            let fixed = rustfix::apply_suggestions(&code, &suggestions)
                .expect("could not apply suggestions");
            fs::write(new_file_name.to_string(), fixed).unwrap();
            true
        };

        repair_iteration(&mut compile_cmd, &process_errors, true, None)
    }

    fn repair_function(&self, file_name: &str, new_file_name: &str, _: &str) -> RepairResult {
        self.repair_file(file_name, new_file_name)
    }
}
