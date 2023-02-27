struct A<'a> {
    x: &'a String,
}
fn new_foo() {
    let x = String::new();
    let a = A { x: &x };
    bar(&x, &a)
}
fn bar<'lt0, 'lt1, 'lt2>(x: &'lt0 String, a: &'lt1 A<'lt2>) {
    println!("{}, {}", &*x, a.x)
}

fn main() {new_foo()}
