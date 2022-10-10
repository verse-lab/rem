mod repair_simple;

fn main() {
    let file_names = vec!["borrow", "in_out_lifetimes", "lifetime_bounds", "in_out_lifetimes_original_extract"];
    for file_name in file_names {
        let success =
            repair_simple::repair_file(
                format!("./input/{}.rs", file_name).as_str(),
                format!("./output/{}.rs", file_name).as_str()
            );
        println!("refactored {}: {}", file_name, success);
    }
}
