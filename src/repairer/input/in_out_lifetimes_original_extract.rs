const W: i32 = 5;

pub fn original_foo() {
    let x = 1;
    let x_ref = &x;
    let mut z: &i32;
    {
        let y = 2;
        z = &y;
        z = extracted(x_ref, &mut z, &y);
        println!("{}", *z);
    }
}

fn extracted(x_ref: &i32, z: &mut &i32, y: &i32) -> &i32 {
    if *z < *x_ref {
        &y
    } else {
        &W
    }
}

fn main() {}
