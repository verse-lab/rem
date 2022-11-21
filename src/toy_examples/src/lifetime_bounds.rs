pub fn original_foo(){
    let x = 1;
    {
        let p : &mut &i32 = &mut &0;
        *p = &x;
        println!("{}", **p);
    }
}

/*
pub fn new_foo(){
    let p : &mut &i32 = &mut &0;
    {
        let x = 1;
        bar_extracted(p, &x);
        println!("{}", **p);
    }
}

fn bar_extracted(p: &mut &i32, x: &i32) {
    *p = &x;
}
*/

pub fn new_foo_fixed(){
    let x = 1;
    {
        let p : &mut &i32 = &mut &0;
        bar_fixed(p, &x);
        println!("{}", **p);
    }
}

fn bar_fixed<'a, 'b: 'a>(p: & 'a mut & 'b i32, x: & 'b i32) {
    *p = &x;
}