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

fn extracted<'a>(x_ref: &'a i32, z: &'a mut &'a i32, y: &'a i32) -> &'a i32 {
    if *z < x_ref {
        &y
    } else {
        &W
    }
}

fn main() {}
