pub trait RepairSystem {
    fn name(&self) -> &str;
    fn repair_file(&self, file_name: &str, new_file_name: &str) -> bool;
}