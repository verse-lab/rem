
pub fn original_foo(){
    let p : &mut &i32 = &mut &0;
    {
        let x = 1;
        *p = &x;
        {
            let p = &mut &0;
            *p = &x;
            {
                let p = &mut &0;
                let x = 2;
                *p = &x;
            }
        }
    }
}


pub fn new_foo(){
    let p : &mut &i32 = &mut &0;
    {
        let x = 1;
        bar(p, &x);
        {
            let p = &mut &0;
            bar(p, &x);
            {
                let p = &mut &0;
                let x = 2;
                bar(p, &x);
            }
        }
    }
}

fn bar<'a, 'b>(p: & 'a mut & 'b i32, x: & 'b i32) {
    *p = &x;
}