pub fn new_foo() {
    let mut z: &i32 = &0;
    let mut y = 2;
    z = bar(z, y);
}

fn bar(z: &i32, y: i32) -> &i32 {
    y = *z + 1;
    &y
}

fn main() {
    new_foo();
}