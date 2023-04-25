pub fn foo() {
    let x = 1;
    println!("x={}", x);
}
#[allow(dead_code)]
pub fn foo_new() {
    let x = 1;
    bar(x);
    let y = 1;
    if y == 2 {
        println!("something")
    }
}
fn bar(x: i32) {
    println!("x={}", x);
}
fn main() {}
