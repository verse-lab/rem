#[allow(dead_code)]
pub struct Foo {x: i32, y: i32}

#[allow(dead_code)]
pub struct FooUpdated<'a, 'b> {x: &'a i32, y: &'b i32}

pub fn make_foo(x: &i32, y: &i32) -> Foo {
    Foo{x: *x, y: *y}
}
pub fn make_foo_updated<'a, 'b>(x: &'a i32, y: &'b i32) -> FooUpdated<'a, 'b>{
    FooUpdated{x, y}
}