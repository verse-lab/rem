pub fn foo(){
    let p : &mut &i32 = &mut &0;
    {
        let x = 1;
        *p = &x;
    }
}

fn main() {
    foo()
}