pub fn extract_read_no_use_after() {
    let x = 1;
    println!("x={}", x);
}
#[allow(dead_code)]
pub fn extract_read_no_use_after_new() {
    let x = 1;
    extract_read_no_use_after_bar(x);
}
fn extract_read_no_use_after_bar(x: i32) {
    println!("x={}", x);
}
pub fn extract_read_use_after() {
    let x = 1;
    println!("x={}", x);
    println!("x={}", x);
}
#[allow(dead_code)]
pub fn extract_read_use_after_new() {
    let x = 1;
    extract_read_use_after_bar(x);
    println!("x={}", x);
    extract_read_no_use_after_bar(x);
}
fn extract_read_use_after_bar(x: i32) {
    let y = x;
    println!("x={}", x);
    extract_read_no_use_after_bar(x);
    let z = y;
    let n = z + x;
}
#[allow(unused_assignments, unused_variables)]
pub fn extract_write_no_use_after() {
    let mut x = 1;
    x = 5;
}
#[allow(dead_code)]
pub fn extract_write_no_use_after_new() {
    let mut x = 1;
    extract_write_no_use_after_bar(&mut x);
}
fn extract_write_no_use_after_bar(x: &mut i32) {
    x = 5;
}
#[allow(unused_assignments)]
pub fn extract_write_use_after() {
    let mut x = 1;
    x = 5;
    println!("x={}", x);
}
#[allow(dead_code)]
pub fn extract_write_use_after_new() {
    let mut x = 1;
    extract_write_use_after_bar(&mut x);
    println!("x={}", x);
}
fn extract_write_use_after_bar(x: &mut i32) {
    x = 5;
}
#[allow(unused_assignments)]
pub fn extract_read_and_write() {
    let mut x = 1;
    x = 5;
    x = 6;
    println!("x={}", x);
    println!("x={}", x);
}
pub fn extract_read_and_write_new() {
    let mut x = 1;
    extract_read_and_write_bar(&mut x);
    println!("x={}", x);
}
#[allow(unused_assignments)]
fn extract_read_and_write_bar(x: &mut i32) {
    x = 5;
    x = 6;
    println!("x={}", x);
}
fn main() {}
