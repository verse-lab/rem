const W: i32 = 5;

pub fn new_foo() {
    let x = 1;
    let x_ref = &x;
    let mut z: &i32;
    {
        let y = 2;
        z = &y;
        z = bar(x_ref, z, y);
        println!("{}", *z);
    }
}

fn bar(x_ref: &i32, z: &i32, y: i32) -> &i32 {
    if *z < *x_ref {
        &y
    } else {
        &W
    }
}

fn main() {}
