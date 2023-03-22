struct A {
    x: Result<(), String>,
}

fn new_foo() -> Result<(), String> {
    let x : Result<(), String> = Ok(());
    bar(x);
    Ok(())
}

fn bar(x: Result<(), String>) {
    let _y = x?;
    let a = A {x: Ok(())};
    match a.x? {
        () => (),
    };
}


fn main() {
    new_foo().unwrap();
}