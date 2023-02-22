fn new_foo () {
    let mut x: String = String::new();
    x.push('a');
    println!("x: {}", x);
}

fn main() {
    new_foo();
}