fn foo() -> i32 {
    let mut x = 1;
    if x < 2 {
        x = 5
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

// semantically incorrect right now but with borrower, the changes will be done by &mut
fn bar(x: i32) {
    if x < 2 {
        x = 5
    } else {
        return -1;
    }
}

fn main() {
    foo();
    new_foo();
}