trait MultiLifetimeTrait<'b, 'a: 'b> {
    fn trait_function(self: &Self, x: & 'a i32, y: &'b i32) -> & 'b i32;
}

struct SimpleStruct;

impl<'b, 'a: 'b> MultiLifetimeTrait<'b, 'a> for SimpleStruct {
    fn trait_function(&self, x: &'a i32, y: &'b i32) -> &'b i32 {
        if *x < *y {
            y
        } else {
            x
        }
    }
}

pub fn original_foo<'b, 'a: 'b>(x: &'a i32, y: &'b i32) {
    let foo = SimpleStruct;
    let z = &mut &0;
    *z = foo.trait_function(x, y);
}

/*
pub fn new_foo<'b, 'a: 'b>(x: &'a i32, y: &'b i32) {
    let foo = SimpleStruct;
    let z = &mut &0;
    *z = bar_extracted(x, y, foo);
}

fn bar_extracted(x: &i32, y: &i32, foo: SimpleStruct) -> &i32 {
    foo.trait_function(x, y)
}
*/

pub fn new_foo_fixed<'b, 'a: 'b>(x: &'a i32, y: &'b i32) {
    let foo = SimpleStruct;
    let z = &mut &0;
    *z = bar_fixed(x, y, foo);
}

fn bar_fixed<'b, 'a: 'b>(x: &'a i32, y: &'b i32, foo: SimpleStruct) -> &'b i32 {
    foo.trait_function(x, y)
}