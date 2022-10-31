extern crate colored;

use std::iter::zip;
use std::time::SystemTime;
use colored::Colorize;

mod common;
mod repair_lifetime_simple;
mod repair_rustfix;
mod repair_lifetime_tightest_bound_first;
mod repair_lifetime_loosest_bound_first;

use common::RepairSystem;

fn main() {
    let file_names = vec!["borrow", "in_out_lifetimes", "lifetime_bounds", "in_out_lifetimes_original_extract", "lifetime_bounds_not_enough_annotations", "in_out_lifetimes_wide_bounds"];
    let function_sigs = vec![("",""), ("bar_extracted", "fn bar_extracted(x_ref: &i32, z: &i32, y: &i32) -> &i32"), ("bar_extracted", "fn bar_extracted(p: &mut & i32, x: & i32)"), ("", ""), ("", ""), ("", "")];
    let repair_systems: Vec<&dyn RepairSystem> = vec![&repair_lifetime_simple::Repairer {}, &repair_rustfix::Repairer {}, &repair_lifetime_tightest_bound_first::Repairer {}, &repair_lifetime_loosest_bound_first::Repairer {}];
    for (file_name, (function_name, function_sig)) in zip(file_names, function_sigs) {
        for repair_system in repair_systems.iter() {
            println!("\n\n{}, {}: {}", file_name, function_name, function_sig);
            let now = SystemTime::now();
            let success =
                repair_system.repair_function(
                    format!("./input/{}.rs", file_name).as_str(),
                    format!("./output/{}{}.rs", file_name, repair_system.name()).as_str(),
                    function_sig,
                    function_name,
                );
            let time_elapsed = now.elapsed().unwrap();
            println!("{}: {} refactored {} in {:#?}",
                     (if success { format!("PASSED").green() } else { format!("FAILED").red() }), repair_system.name(), file_name, time_elapsed);
        }
        println!("------------------------------------------------------------------");
    }
}
