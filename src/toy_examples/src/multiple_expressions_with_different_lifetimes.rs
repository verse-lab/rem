pub fn original_foo1<'a, 'b : 'a> (p: &'a mut &'b i32, x: &'b i32) where 'a : 'b{
    *p = x;
}

pub fn original_foo2<'a, 'b : 'a>(p: &'a mut &'b i32, x: &'b i32){
    *p = x;
}

/*
pub fn new_foo1<'a, 'b : 'a> (p: &'a mut &'b i32, x: &'b i32) where 'a : 'b {
    bar_extracted(p, x);
}

fn bar_extracted(p: &mut &i32, x: &i32) {
    *p = x;
}
*/

pub fn new_foo1_fixed<'a, 'b : 'a> (p: &'a mut &'b i32, x: &'b i32) where 'a : 'b {
    bar_fixed(p, x);
}

/*
pub fn new_foo2<'a, 'b : 'a>(p: &'a mut &'b i32, x: &'b i32){
    bar_fixed(p, x);
}
 */

fn bar_fixed<'a, 'b : 'a>(p: & 'a mut & 'b i32, x: & 'b i32) where 'a : 'b {
    *p = &x;
}