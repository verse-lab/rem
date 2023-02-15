pub fn original_foo() {
    let p: &mut &i32 = &mut &0;
    let x = 1;
    *p = &x;
}

pub fn new_foo() {
    let p: &mut &i32 = &mut &0;
    let x = 1;
    bar_extracted(p, &x);
    println!("{}", **p);
}

fn bar_extracted(p: &mut &i32, x: &i32) {
    *p = &x;
}

fn main() {}
