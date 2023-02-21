fn new_foo() {
    let mut a = vec![1, 2, 3];
    let mut b = vec![5, 2, 3];
    let x = 1;
    let y = 2;
    println!("{}{}", x, y);
    a.push(4);
    let z = &x;
    bar(&mut a, x);
    let _ = a.get(0);
    b[0] = a[0];
    println!("{}{}", a[0], b[0]);
    println!("x={}", z);
}
fn bar(a: &mut Vec<i32>, x: i32) {
    a.push(x);
}
fn main() {
    new_foo()
}
