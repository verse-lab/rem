fn new_foo() -> i32 {
    let mut x = 7;
    let y = 11;
    while y > 1 {
        match bar(y) {
            RetBar::Ok(x) => x,
            RetBar::Continue => continue,
        };
        x -= 1;
    }
    x
}
fn bar(y: i32) -> RetBar<()> {
    let result = if y == 5 {
        return RetBar::Continue;
    };
    RetBar::Ok(result)
}
fn main() {
    new_foo();
}
enum RetBar<A> {
    Ok(A),
    Continue,
}
