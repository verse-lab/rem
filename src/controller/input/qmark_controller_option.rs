struct A {
    x: Option<()>,
}

fn new_foo() -> Option<()> {
    let x = Some(());
    bar(x);
    Ok(())
}

fn bar(x: Option<()>) {
    let _y = x?;
    let a = A {x: None};
    match a.x? {
        () => (),
    };
}


fn main() {
    new_foo().unwrap();
}