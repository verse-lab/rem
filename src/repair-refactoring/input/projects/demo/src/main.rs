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
    struct_use();
}