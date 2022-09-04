

fn original_foo() -> String {
    let x = 1;
    let y = if x > 2 {
        x
    } else {
        return String::from("y")
    };
    String::from(y)
}

fn new_foo() -> String {
    let x = 1;
    let y = bar(x);
    String::from(y)
}

fn bar_extracted(x: i32) -> i32 {
    if x > 2 {
        x
    } else {
        return String::from("y")
    }
}
