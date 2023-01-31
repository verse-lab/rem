fn foo() -> i32 {
    let x = 1;
    let y = if x < 2 {
        5
    } else {
        return -1;
    };
    y
}
fn new_foo() -> i32 {
    let x = 1;
    let y = match bar(x) {
        RetBar::Ok(x) => x,
        RetBar::Return(x) => return x,
    };
    y
}
fn bar(x: i32) -> RetBar<i32, i32> {
    let result = if x < 2 {
        5
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
