pub fn lt_bounds(){
    let p : &mut &i32 = &mut &0;
    {
        let x = 1;
        *p = &x;
    }
}

pub fn lt_bounds_use(){
    let p : &mut &i32 = &mut &0;
    {
        let x = 1;
        *p = &x;
        println!("{}", **p);
    }
}

const W: i32 = 5;

// does not work because improper ref handlings
pub fn in_out_lt () {
    let x = 1;
    let x_ref = &x;
    let mut z : &i32;
    {
        let y = 2;
        z = &y;
        z = if *z < *x_ref {
            &y
        } else {
            &W
        };
        println!("{}", *z);
    }
}

struct X {
    x: i32
}

// does not work because no dependency
pub fn struct_use() {
    let x = X { x: 2};
    let x = x.x;
    {
        let p : &mut &i32 = &mut &0;
        *p = &x;
    }
    println!("{}", x);
}

fn main() {
    lt_bounds();
    lt_bounds_use();
    in_out_lt();
    struct_use();
}
