fn new_foo() -> i32 {
    let mut x = 7;
    let y = 11;
    while y > 1 {
        bar(y);
        x -= 1;
    }
    x
}

fn bar(y: i32) {
    if y == 5 {
        continue
    }
}

fn main() {
    new_foo();
}