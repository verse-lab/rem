// 1. original
pub fn extract_read_no_use_after() {
    let x = 1;
    println!("x={}", x);
}

// 1. new
#[allow(dead_code)]
pub fn extract_read_no_use_after_new() {
    let x = 1;
    extract_read_no_use_after_bar(x);
}

// 1. extracted
fn extract_read_no_use_after_bar(x: i32) {
    println!("x={}", x);
}

// 2. original
pub fn extract_read_use_after() {
    let x = 1;
    println!("x={}", x);
    println!("x={}", x);
}

// 2. new
#[allow(dead_code)]
pub fn extract_read_use_after_new() {
    let x = 1;
    extract_read_use_after_bar(x);
    println!("x={}", x);
}

// 2. extracted
fn extract_read_use_after_bar(x: i32) {
    println!("x={}", x);
}

// 3. original
#[allow(unused_assignments, unused_variables)]
pub fn extract_write_no_use_after() {
    let mut x = 1;
    x = 5;
}

// 3. new
#[allow(dead_code)]
pub fn extract_write_no_use_after_new() {
    let mut x = 1;
    extract_write_no_use_after_bar_fixed(&mut x);
}

// 3. extracted
/*
fn extract_write_no_use_after_bar(x: &mut i32) {
    x = 5;
}
*/

fn extract_write_no_use_after_bar_fixed(x: &mut i32) {
    *x = 5;
}

// 4. original
#[allow(unused_assignments)]
pub fn extract_write_use_after() {
    let mut x = 1;
    x = 5;
    println!("x={}", x);
}

// 4. new
#[allow(dead_code)]
pub fn extract_write_use_after_new() {
    let mut x = 1;
    extract_write_use_after_bar_fixed(&mut x);
    println!("x={}", x);
}

// 4. extracted
/*
fn extract_write_use_after_bar(x: &mut i32) {
    x = 5;
}
*/
fn extract_write_use_after_bar_fixed(x: &mut i32) {
    *x = 5;
}

// 5. original
#[allow(unused_assignments)]
pub fn extract_read_and_write() {
    let mut x = 1;
    x = 5;
    println!("x={}", x);
    println!("x={}", x);
}

// 5. new
pub fn extract_read_and_write_new() {
    let mut x = 1;
    extract_read_and_write_bar_fixed(&mut x);
    println!("x={}", x);
}

// 5. extracted
/*
fn extract_read_and_write_bar(x: &mut i32) {
    x = 5;
    println!("x={}", x);
}
*/

fn extract_read_and_write_bar_fixed(x: &mut i32) {
    *x = 5;
    println!("x={}", x);
}

pub fn original_foo() {
    let mut x = 1;
    println!("x={}", x);
    x = 5;
    println!("x={}", x);
}

pub fn new_foo() {
    let mut x = 1;
    extract_immutable(&x);
    let x_ref = &mut x;
    extract_mutable(x_ref);
    println!("x={}", x);
}

fn extract_immutable(x: &i32) {
    println!("x={}", *x);
}

fn extract_mutable(x: &mut i32) {
    *x = 5;
}
