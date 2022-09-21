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
    *z = bar_extracted(foo, x, y);
}

fn bar_extracted(foo: SimpleStruct, x: &i32, y: &i32) -> &i32 {
    foo.trait_function(x, y)
}
*/