pub fn new_foo() {
    let mut z: &i32 = &0;
    let mut y = 2;
    z ={
        y = *z + 1;
        &y
    };
}

fn main() {
    new_foo();
}