struct A {
    x: Result<(), String>,
}
fn new_foo() -> Result<(), String> {
    let x: Result<(), String> = Ok(());
    match bar(x) {
        RetBar::Ok(x) => x,
        RetBar::Return(x) => return x,
    };
    Ok(())
}
fn bar(x: Result<(), String>) -> RetBar<(), Result<(), String>> {
    let _y = match x {
        Ok(x) => x,
        Err(e) => return RetBar::Return(Err(e)),
    };
    let a = A { x: Ok(()) };
    match match a.x {
        Ok(x) => x,
        Err(e) => return RetBar::Return(Err(e)),
    } {
        () => (),
    };
    RetBar::Ok(())
}
fn main() {
    new_foo().unwrap();
}
enum RetBar<A, B> {
    Ok(A),
    Return(B),
}
