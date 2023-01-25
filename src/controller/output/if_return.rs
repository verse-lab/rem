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
    let y = bar(x);
    y
}
fn bar(x: i32) -> Ret_bar<i32, i32> {
    let result = if x < 2 {
        5
    } else {
        return Ret_bar::Return(-1);
    };
    Ret_bar::Ok(result)
}
fn main() {
    foo();
    new_foo();
}
enum Ret_bar<A, B> {
    Ok(A),
    Return(B),
}
