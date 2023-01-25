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
fn bar_extracted<'lt0, 'lt1>(x_ref: &i32, z: &i32, y: &'lt0 i32) -> &'lt1 i32
where
    'lt0: 'lt1,
{
    if *z < *x_ref {
        &y
    } else {
        &W
    }
}
fn main() {}