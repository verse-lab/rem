// 4. original
#[allow(unused_assignments, dead_code)]
pub fn foo() {
    let mut x = 1;
    x = 5;
    println!("x={}", x);
}

// 4. new
fn helper(x: i32){
    println!("x={}", x);
}
#[allow(dead_code)]
pub fn new_foo() {
    let mut x = 1;
    bar(x);
    println!("x={}", x);
    if x == 5 {
        helper(x)
    }
}

// 4. extracted
fn bar(x: i32) {
    x = 5;
}

fn main() {}
