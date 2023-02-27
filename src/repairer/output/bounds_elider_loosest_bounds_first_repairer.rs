struct A<'a> {
    x: &'a String,
}
fn new_foo() {
    let x = String::new();
    let a = A { x: &x };
    bar(&x, &a)
}
fn bar(x: &String, a: &A) {
    println!("{}, {}", &*x, a.x)
}
fn main() {
    new_foo()
}
