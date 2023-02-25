fn new_foo(x: &mut i32) -> String {
    loop {
        let y = *x;
        match bar(x, y) {
            RetBar::Ok(x) => x,
            RetBar::Return(x) => return x,
            RetBar::Break => break,
        }
    }
    x.to_string()
}
fn bar(x: &mut i32, y: i32) -> RetBar<(), String> {
    let result = if y > 2 {
        *x = y - 1;
    } else if y == 1 {
        return RetBar::Return(String::new());
    } else {
        return RetBar::Break;
    };
    RetBar::Ok(result)
}
fn main() {
    let mut x = 1;
    new_foo(&mut x);
}
enum RetBar<A, B> {
    Ok(A),
    Return(B),
    Break,
}
