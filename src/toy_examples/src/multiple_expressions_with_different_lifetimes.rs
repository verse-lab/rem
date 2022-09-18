
pub fn original_foo1<'a, 'b : 'a> (p: &'a mut &'a i32, x: &'b i32) where 'a : 'b{
    *p = x;
}

pub fn original_foo2<'a, 'b : 'a>(p: &'a mut &'b i32, x: &'b i32){
    *p = x;
}

pub fn new_foo1<'a, 'b : 'a> (p: &'a mut &'a i32, x: &'b i32) where 'a : 'b {
    bar(p, x);
}

pub fn new_foo2<'a, 'b : 'a>(p: &'a mut &'b i32, x: &'b i32){
    bar(p, x);
}

fn bar<'a, 'b : 'a>(p: & 'a mut & 'b i32, x: & 'b i32) where 'a : 'b {
    *p = &x;
}