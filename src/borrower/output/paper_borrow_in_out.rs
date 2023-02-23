pub fn new_foo() {
    let mut z: &i32 = &0;
    let mut y = 2;
    z = bar(z, &mut y);
}
fn bar(z: &i32, y: &mut i32) -> &i32 {
    *y = *z + 1;
    &*y
}
fn main() {
    new_foo();
}
