const W: i32 = 5;

pub fn original_foo () {
    let x = 1;
    let x_ref = &x;
    let mut z : &i32;
    {
        let y = 2;
        z = &y;
        z = if *z < *x_ref {
            &y
        } else {
            &W
        };
        println!("{}", *z);
    }
}

/*
pub fn new_foo () {
    let x = 1;
    let x_ref = &x;
    let mut z : &i32;
    {
        let y = 2;
        z = &y;
        z = bar_extracted(z, x_ref, y);
        println!("{}", *z);
    }
}

fn bar_extracted(z: &i32, x_ref: &i32, y: i32) -> &i32 {
    if *z < *x_ref {
        &y
    } else {
        &W
    }
}
*/