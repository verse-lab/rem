use std::time::SystemTime;

mod repair_lifetime_simple;
mod repair_rustfix;
mod repair_system;

fn main() {
    let file_names = vec!["borrow", "in_out_lifetimes", "lifetime_bounds", "in_out_lifetimes_original_extract"];
    let repair_systems = vec![repair_lifetime_simple.repairer];
    for file_name in file_names {
        for repair_system in repair_systems.iter() {
            let now = SystemTime::now();
            let success =
                repair_system(
                    format!("./input/{}.rs", file_name).as_str(),
                    format!("./output/{}.rs", file_name).as_str()
                );
            let time_elapsed = now.elapsed().unwrap();
            println!("{} refactored {} success: {} in {:#?}",
                     std::stringify!(repair_system), file_name, success, time_elapsed);
        }
    }
}
