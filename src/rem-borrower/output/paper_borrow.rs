fn new_foo() {
    let mut x: String = String::new();
    bar(&mut x);
    println!("x: {}", x);
}
fn bar(x: &mut String) {
    x.push('a')
}
fn main() {
    new_foo();
}
