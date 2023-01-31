fn new_foo() -> i32 {
    let x = 7;
    let mut y = 11;
    while y > 1 {
        y = bar(x);
    }
    y
}

fn bar(x: i32) -> i32 {
    if x == 5 {
        break
    } else {
        x- 1
    }
}

fn main() {
    new_foo();
}