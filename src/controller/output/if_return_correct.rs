enum RetNewFoo<A,B> {
    Ok(A),
    Return(B),
}

fn foo() -> i32 {
    let x = 1;
    let y = if x < 2 {
        5
    } else {
        return -1;
    };
    y
}

fn new_foo() -> i32 {
    let x = 1;
    let y = match bar(x) {
        RetNewFoo::Ok(ok) => ok,
        RetNewFoo::Return(r) => return r,
    };
    y
}

fn bar(x: i32) -> RetNewFoo<i32,i32> {
    let result = if x < 2 {
        5
    } else {
        return RetNewFoo::Return(-1)
    };
    RetNewFoo::Ok(result)
}

fn main() {
    foo();
    new_foo();
}