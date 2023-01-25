fn foo() -> i32 {
    let x = 1;
    let y = if x < 2 {
        5
    } else {
        return -1;
    };
    y
}

fn new_foo() -> i32 {
    let x = 1;
    let y = bar(x);
    y
}

fn bar(x: i32) -> i32 {
    if x < 2 {
        5
    } else {
        return -1;
    }
}

fn main() {
    foo();
    new_foo();
}