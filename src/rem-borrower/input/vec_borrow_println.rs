#[allow(dead_code)]
fn foo() {
    let mut x = vec![];
    x.push(0);
    x[0] = 1;
    println!("something {}", x[0]);
}

#[allow(dead_code)]
fn new_foo() {
    let mut x = vec![];
    x.push(0);
    bar(x);
    println!("something {}", x[0]);
}
fn bar(x: Vec<i32>) {
    x[0] = 1;
}
fn main() {}
