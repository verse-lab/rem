struct A<'a> {
    x: &'a String,
}
fn new_foo() {
    let x = String::new();
    let a = A { x: &x };
    let _ = bar(&x, &a);
}
fn bar<'lt0, 'lt1, 'lt2>(x: &'lt0 String, a: &A) -> Result<A<'lt2>, String>
where
    'lt0: 'lt2,
{
    println!("{}, {}", &*x, a.x);
    Ok(A { x })
}
fn main() {
    new_foo()
}
