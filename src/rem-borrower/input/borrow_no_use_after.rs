// 1. original
pub fn foo() {
    let x = 1;
    println!("x={}", x);
}

// 1. new
#[allow(dead_code)]
pub fn foo_new() {
    let x = 1;
    bar(x);
    let y = 1;
    if y == 2 {
        println!("something")
    }
}

// 1. extracted
fn bar(x: i32) {
    println!("x={}", x);
}

fn main() {}
