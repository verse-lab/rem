const W: i32 = 5;

pub fn new_foo() {
    let x = 1;
    let x_ref = &x;
    let mut z: &i32;
    {
        let y = 2;
        z = &y;
        z = if *z < *x_ref {
            &y
        } else {
            &W
        };
        println!("{}", y);
        println!("{}", *z);
    }
}


fn main() {}
