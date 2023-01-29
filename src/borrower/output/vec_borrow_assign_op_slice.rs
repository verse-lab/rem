#[allow(dead_code)]
fn foo() {
    let mut x = vec![];
    x.push(0);
    x[0] += 1;
    if x[0] > 1 {
        println!("something")
    }
}
#[allow(dead_code)]
fn new_foo() {
    let mut x = vec![];
    x.push(0);
    bar(&mut x);
    if x[0] > 1 {
        println!("something")
    }
}
fn bar(x: &mut Vec<i32>) {
    x[0] += 1;
}
fn main() {}
