#[allow(dead_code)]
pub fn foo() {
    let x = 1;
    let y = x;
    println!("x={}", x);
    helper(x);
    let z = y;
    let _n = z + x;
    println!("x={}", x);
    helper(x);
}
fn helper(x: i32) {
    println!("{}", x);
}
#[allow(dead_code)]
pub fn new_foo() {
    let x = 1;
    bar(&x);
    println!("x={}", x);
    helper(x);
}
fn bar(x: &i32) {
    let y = *x;
    println!("x={}", x);
    helper(*x);
    let z = y;
    let _n = z + *x;
}
fn main() {}
