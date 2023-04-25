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
    match bar(x) {
        RetBar::Ok(x) => x,
        RetBar::Return(x) => return x,
    };
    x
}
fn bar(x: i32) -> RetBar<(), i32> {
    let result = if x < 2 {
        x = 5
    } else {
        return RetBar::Return(-1);
    };
    RetBar::Ok(result)
}
fn main() {
    foo();
    new_foo();
}
enum RetBar<A, B> {
    Ok(A),
    Return(B),
}
