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
        bar_extracted(p, &x);
        println!("{}", **p);
    }
}

fn bar_extracted<'lt0, 'lt1, 'lt2, 'lt3, 'lt4>(p: &'lt4 mut &'lt4  i32, x: &'lt0  i32)  where 'lt0: 'lt1, 'lt0: 'lt2, 'lt0: 'lt3 where 'lt0: 'lt4 {
    *p = &x;
}

fn main() {}















