use std::collections::HashSet;
use std::fs;
use std::process::Command;
use crate::repair_system::RepairSystem;

fn compile_file(file_name: &str) -> Command {
    let mut compile = Command::new("rustc");
    compile
        .arg("--error-format=json")
        .arg(file_name);
    compile
}
pub struct Repairer {}

impl RepairSystem for Repairer {
    fn name(&self) -> &str {
        "_rustfix_repairer"
    }

    fn repair_file(&self, file_name: &str, new_file_name: &str) -> bool {
        fs::copy(file_name, &new_file_name).unwrap();

        loop {
            let out = compile_file(&new_file_name).output().unwrap();
            let stderr = String::from_utf8_lossy(&out.stderr);
            if stderr.len() == 0 {
                return true;
            }

            // println!("compile stdout: {}", String::from_utf8_lossy(&out.stdout));
            // println!("compile stderr: {}", stderr);

            let suggestions =
                rustfix::get_suggestions_from_json(&*stderr, &HashSet::new(), rustfix::Filter::Everything)
                    .expect("rustfix failed to run on error json");

            if suggestions.len() == 0 {
                break false;
            }

            let code: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();
            let fixed = rustfix::apply_suggestions(&code, &suggestions)
                .expect("could not apply suggestions");
            fs::write(new_file_name.to_string(), fixed).unwrap();
        }
    }
}
