struct A {
    x: Option<()>,
}

fn new_foo() -> Option<()> {
    let x = Some(());
    bar(x)
}

fn bar(x: Option<()>) -> Option<()> {
    let _y = x?;
    let a = A { x: None };
    match a.x? {
        () => Some(()),
    }
}


fn main() {
    new_foo().unwrap();
}