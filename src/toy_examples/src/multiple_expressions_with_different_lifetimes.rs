pub fn original_foo1<'a>(p: &'a mut &'a i32, x: &'a i32) {
    *p = x;
}

pub fn original_foo2<'a, 'b: 'a>(p: &'a mut &'b i32, x: &'b i32) {
    *p = x;
}

/*
pub fn new_foo1<'a> (p: &'a mut &'a i32, x: &'a i32){
    bar_extracted(p, x);
}

fn bar_extracted(p: &mut &i32, x: &i32) {
    *p = x;
}
*/

pub fn new_foo1_fixed<'a>(p: &'a mut &'a i32, x: &'a i32) {
    bar_fixed(p, x);
}

/*
pub fn new_foo2<'a, 'b : 'a>(p: &'a mut &'b i32, x: &'b i32){
    bar_fixed(p, x);
}
 */

fn bar_fixed<'a>(p: &'a mut &'a i32, x: &'a i32) {
    *p = x;
}
