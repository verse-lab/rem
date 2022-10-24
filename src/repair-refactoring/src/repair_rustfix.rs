use std::fs;
use std::process::Command;
use crate::repair_system::RepairSystem;

fn compile_file(file_name: &str) -> Command {
    let mut compile = Command::new("rustc");
    compile
        .arg(file_name);
    compile
}
pub struct Repairer {}

impl RepairSystem for Repairer {
    fn name(&self) -> &str {
        "rustfix repairer"
    }

    fn repair_file(&self, file_name: &str, new_file_name: &str) -> bool {
        
    }
}