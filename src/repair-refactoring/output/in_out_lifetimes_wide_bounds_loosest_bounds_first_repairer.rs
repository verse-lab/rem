const W: i32 = 5;
pub fn original_foo() {
    let x = 1;
    let x_ref = &x;
    let mut z: &i32;
    {
        let y = 2;
        z = &y;
        z = if *z < *x_ref { &y } else { &W };
        println!("{}", *z);
    }
}
pub fn new_foo() {
    let x = 1;
    let x_ref = &x;
    let mut z: &i32;
    {
        let y = 2;
        z = &y;
        z = bar_extracted(x_ref, z, &y);
        println!("{}", *z);
    }
    z = x_ref;
    println!("{}", *z);
}
fn bar_extracted<'lt0, 'lt1, 'lt2>(x_ref: &'lt0 i32, z: &'lt1 i32, y: &'lt2 i32) -> &'lt0 i32
where
    'lt2: 'lt0,
    'lt1: 'lt0,
{
    if *z < *x_ref {
        &y
    } else {
        z
    }
}
fn main() {}
