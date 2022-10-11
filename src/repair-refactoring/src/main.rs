use std::time::SystemTime;

mod repair_lifetime_simple;

fn main() {
    let file_names = vec!["borrow", "in_out_lifetimes", "lifetime_bounds", "in_out_lifetimes_original_extract"];
    for file_name in file_names {
        let now = SystemTime::now();
        let success =
            repair_lifetime_simple::repair_file(
                format!("./input/{}.rs", file_name).as_str(),
                format!("./output/{}.rs", file_name).as_str()
            );
        let time_elapsed = now.elapsed().unwrap();
        println!("refactored {} success: {} in {:#?}", file_name, success, time_elapsed);
    }
}
