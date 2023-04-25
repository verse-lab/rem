#[allow(unused_assignments, dead_code)]
pub fn foo() {
    let mut x = 1;
    x = 5;
    println!("x={}", x);
}
fn helper(x: i32) {
    println!("x={}", x);
}
#[allow(dead_code)]
pub fn new_foo() {
    let mut x = 1;
    bar(&mut x);
    println!("x={}", x);
    if x == 5 {
        helper(x)
    }
}
fn bar(x: &mut i32) {
    *x = 5;
}
fn main() {}
