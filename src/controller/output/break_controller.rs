fn new_foo() -> i32 {
    let x = 7;
    let mut y = 11;
    while y > 1 {
        y = match bar(x) {
            RetBar::Ok(x) => x,
            RetBar::Break => break,
        };
    }
    y
}
fn bar(x: i32) -> RetBar<i32> {
    let result = if x == 5 { return RetBar::Break } else { x - 1 };
    RetBar::Ok(result)
}
fn main() {
    new_foo();
}
enum RetBar<A> {
    Ok(A),
    Break,
}
