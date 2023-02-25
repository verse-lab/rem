fn new_foo(x: &mut i32) -> String {
    loop {
        let y = *x;
        bar(x, y)
    }
    x.to_string()
}

fn bar(x: &mut i32, y: i32) {
    if y > 2 {
        *x = y - 1;
    } else if y == 1 {
        return String::new();
    }
    else {
        break;
    }
}

fn main() {
    let mut x = 1;
    new_foo(&mut x);
}