// 2. original
#[allow(dead_code)]
pub fn new_foo() {
    let x = 1;
    let y = x;
    println!("x={}", x);
    helper(x);
    let z = y;
    let _n = z + x;
    println!("x={}", x);
    helper(x);
}

// 2. new
fn helper(x: i32) {
    println!("{}",x);
}

fn main() {}
