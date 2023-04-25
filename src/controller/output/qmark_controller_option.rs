struct A {
    x: Option<()>,
}
fn new_foo() -> Option<()> {
    let x = Some(());
    match bar(x) {
        RetBar::Ok(x) => x,
        RetBar::Return(x) => return x,
    }
}
fn bar(x: Option<()>) -> RetBar<Option<()>, Option<()>> {
    let _y = match x {
        Some(x) => x,
        None => return RetBar::Return(None),
    };
    let a = A { x: None };
    let result = match match a.x {
        Some(x) => x,
        None => return RetBar::Return(None),
    } {
        () => Some(()),
    };
    RetBar::Ok(result)
}
fn main() {
    new_foo().unwrap();
}
enum RetBar<A, B> {
    Ok(A),
    Return(B),
}
