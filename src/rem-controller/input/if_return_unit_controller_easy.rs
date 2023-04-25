fn foo() -> i32 {
    let mut x = 1;
    if x < 2 {
        println!("{}", x);
    } else {
        return -1;
    };
    x
}

fn new_foo() -> i32 {
    let mut x = 1;
    bar(x);
    x
}

fn bar(x: i32) {
    if x < 2 {
        println!("{}", x);
    } else {
        return -1;
    }
}

fn main() {
    foo();
    new_foo();
}