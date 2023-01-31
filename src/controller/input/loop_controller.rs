fn new_foo() -> i32 {
    let mut x = 7;
    let y = 11;
    loop {
        bar(y);
        x -= 1;
    }
    x
}

fn bar(y: i32) {
    if y == 5 {
        continue
    } else { break }
}

fn main() {
    new_foo();
}