pub fn original_foo() {
    let p: &mut &i32 = &mut &0;
    {
        let x = 1;
        *p = &x;
    }
}
pub fn new_foo() {
    let p: &mut &i32 = &mut &0;
    {
        let x = 1;
        bar_extracted(p, &x);
        println!("{}", **p);
    }
}
fn bar_extracted<'lt0, 'lt1, 'lt2>(p: &'lt2 mut &i32, x: &'lt0 i32) {
    *p = &x;
}
fn main() {}









