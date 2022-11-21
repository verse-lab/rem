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

pub fn new_foo_fixed () {
    let x = 1;
    let x_ref = &x;
    let mut z : &i32;
    {
        let y = 2;
        z = &y;
        z = bar_fixed(x_ref, z, &y);
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
        z = bar_extracted(x_ref, z, &y);
        println!("{}", *z);
    }
}

fn bar_extracted(x_ref: &i32, z: &mut &i32, y: &i32) -> &i32 {
    if *z < *x_ref {
        &y
    } else {
        &W
    }
}
*/

fn bar_fixed<'a, 'b>(x_ref: & 'a i32, z: & 'a i32, y: &'b i32) -> & 'b i32 {
    if *z < *x_ref {
        y
    } else {
        &W
    }
}

fn bar_extracted<'a, 'b, 'c>(x_ref: &'a i32, z: &'b i32, y: &'c i32) -> &'c i32 {
    if *z < *x_ref {
        &y
    } else {
        &W
    }
}

fn bar_extracted_elidded<'c>(x_ref: &i32, z: &i32, y: &'c i32) -> &'c i32 {
    if *z < *x_ref {
        &y
    } else {
        &W
    }
}