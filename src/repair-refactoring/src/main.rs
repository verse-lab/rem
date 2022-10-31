use std::iter::zip;
use std::time::SystemTime;

mod common;
mod repair_lifetime_simple;
mod repair_rustfix;
mod repair_lifetime_tightest_bound;

use common::RepairSystem;

fn main() {
    let file_names = vec!["borrow", "in_out_lifetimes", "lifetime_bounds", "in_out_lifetimes_original_extract", "lifetime_bounds_not_enough_annotations", "in_out_lifetimes_wide_bounds"];
    let function_sigs = vec!["", "fn bar_extracted(x_ref: &i32, z: &i32, y: &i32) -> &i32", "fn bar_extracted(p: &mut & i32, x: & i32)", "", "", ""];
    let repair_systems : Vec<&dyn RepairSystem> = vec![&repair_lifetime_simple::Repairer{}, &repair_rustfix::Repairer{}, &repair_lifetime_tightest_bound::Repairer{}];
    for (file_name, function_sig) in zip(file_names, function_sigs) {
        for repair_system in repair_systems.iter() {
            let now = SystemTime::now();
            let success =
                repair_system.repair_function(
                    format!("./input/{}.rs", file_name).as_str(),
                    format!("./output/{}{}.rs", file_name, repair_system.name()).as_str(),
                    function_sig,
                );
            let time_elapsed = now.elapsed().unwrap();
            println!("{} refactored {} success: {} in {:#?}",
                     repair_system.name(), file_name, success, time_elapsed);
        }
    }
}
