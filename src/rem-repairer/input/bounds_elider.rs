struct A<'a> {
    x: &'a String,
}
fn new_foo() {
    let x = String::new();
    let a = A { x: &x };
    let _ = bar(&x, &a);
}
fn bar<'lt0, 'lt1, 'lt2, 'lt3, 'lt4>(x: &'lt0 String, a: &'lt1 A<'lt2>) -> Result<A<'lt4>, String>
where 'lt0: 'lt4{
    println!("{}, {}", &*x, a.x);
    Ok(A {x})
}

fn main() {new_foo()}
