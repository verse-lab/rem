fn new_foo () {
    let mut x: String = String::new();
    bar(x);
    println!("x: {}", x);
}

fn bar (x: String) {
    x.push('a')
}

fn main() {
    new_foo();
}