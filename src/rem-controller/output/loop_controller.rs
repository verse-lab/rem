fn new_foo() -> i32 {
    let mut x = 7;
    let y = 11;
    loop {
        x -= 1;
        match bar(y) {
            RetBar::Ok(x) => x,
            RetBar::Break => break,
            RetBar::Continue => continue,
        };
    }
    x
}
fn bar(y: i32) -> RetBar<()> {
    let result = if y == 5 {
        return RetBar::Continue;
    } else {
        return RetBar::Break;
    };
    RetBar::Ok(result)
}
fn main() {
    new_foo();
}
enum RetBar<A> {
    Ok(A),
    Break,
    Continue,
}
