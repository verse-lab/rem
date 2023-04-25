fn new_foo() -> i32 {
    let mut x = 7;
    let y = 11;
    loop {
        x -= 1;
        bar(y);
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