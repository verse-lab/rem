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

fn bar_extracted<'a>(x_ref: &'a i32, z: &'a i32, y: &'a i32) -> &'a i32 {
    if *z < *x_ref {
        &y
    } else {
        &W
    }
}

fn main() {}
