use std::borrow::Cow;
use std::fs;
use std::vec::IntoIter;

use crate::repair_system::RepairSystem;
use crate::common;

pub struct Repairer {}

impl RepairSystem for Repairer {
    fn name(&self) -> &str {
        "_simple_repairer"
    }

    fn repair_file(&self, file_name: &str, new_file_name: &str) -> bool {
        let args : Vec<&str> = vec![];
        fs::copy(file_name, &new_file_name).unwrap();

        let mut compile_cmd = common::compile_file(&new_file_name, &args);

        let process_errors = |stderr: &Cow<str>| {
            let help_lines = common::lift_general_help(stderr);
            let _x = common::lift_lifetime_constraint(stderr);

            if help_lines.len() == 0 {
                return false;
            }

            let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();
            // println!("{}", file_content);
            let lines = file_content.split("\n");
            let mut lines_modifiable = Vec::new();
            for (_, line) in lines.enumerate() {
                lines_modifiable.push(line);
            }

            for (line_number, line_text) in help_lines {
                lines_modifiable[line_number - 1] = line_text;
            }

            fs::write(new_file_name.to_string(), lines_modifiable.join("\n")).unwrap();
            true
        };

        common::repair_iteration(&mut compile_cmd, &process_errors)
    }
}