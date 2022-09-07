#[allow(dead_code)]

pub fn original_foo(){
    let p : &mut &i32 = &mut &0;
    {
        let x = 1;
        *p = &x;
    }
}

pub fn new_foo(){
    let p : &mut &i32 = &mut &0;
    {
        let x = 1;
        bar_fixed(p, &x);
        println!("{}", **p);
    }
}

/*
fn bar(p: &mut &i32, x: &i32) {
    *p = &x;
}
*/

fn bar_fixed<'a, 'b>(p: & 'a mut & 'b i32, x: & 'b i32) {
    *p = &x;
}