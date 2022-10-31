use std::borrow::Cow;
use std::collections::HashSet;
use std::fs;
use crate::common::{
    RepairSystem, compile_file, repair_iteration
};

pub struct Repairer {}

impl RepairSystem for Repairer {
    fn name(&self) -> &str {
        "_rustfix_repairer"
    }

    fn repair_file(&self, file_name: &str, new_file_name: &str) -> bool {
        fs::copy(file_name, &new_file_name).unwrap();
        let args = vec!["--error-format=json"];

        let mut compile_cmd = compile_file(&new_file_name, &args);

        let process_errors = | stderr : &Cow<str> | {
            let suggestions =
                rustfix::get_suggestions_from_json(&*stderr, &HashSet::new(), rustfix::Filter::Everything)
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

        repair_iteration(&mut compile_cmd, &process_errors, true)
    }

    fn repair_function(&self, file_name: &str, new_file_name: &str, _: &str) -> bool {
        self.repair_file(file_name, new_file_name)
    }
}
