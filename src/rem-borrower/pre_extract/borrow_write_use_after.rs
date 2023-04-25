// 4. new
fn helper(x: i32){
    println!("x={}", x);
}
#[allow(dead_code)]
pub fn new_foo() {
    let mut x = 1;
    x=5;
    println!("x={}", x);
    if x == 5 {
        helper(x)
    }
}

fn main() {}
