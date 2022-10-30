use std::process::Command;
use std::time::SystemTime;
use crate::repair_system::RepairSystem;

mod repair_lifetime_simple;
mod repair_rustfix;
mod repair_system;
mod common;

fn main() {
    let file_names = vec!["borrow", "in_out_lifetimes", "lifetime_bounds", "in_out_lifetimes_original_extract", "lifetime_bounds_not_enough_annotations", "in_out_lifetimes_wide_bounds"];
    //let file_names = vec!["lifetime_bounds_not_enough_annotations"];
    let repair_systems : Vec<&dyn RepairSystem> = vec![&repair_lifetime_simple::Repairer{}, &repair_rustfix::Repairer{}];
    for file_name in file_names {
        for repair_system in repair_systems.iter() {
            let now = SystemTime::now();
            let success =
                repair_system.repair_file(
                    format!("./input/{}.rs", file_name).as_str(),
                    format!("./output/{}{}.rs", file_name, repair_system.name()).as_str()
                );
            let time_elapsed = now.elapsed().unwrap();
            println!("{} refactored {} success: {} in {:#?}",
                     repair_system.name(), file_name, success, time_elapsed);
        }
    }
}
