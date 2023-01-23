pub fn lt_bounds() {
    let p: &mut &i32 = &mut &0;
    {
        let x = 1;
        *p = &x;
    }
}

pub fn lt_bounds_use() {
    let p: &mut &i32 = &mut &0;
    {
        let x = 1;
        bar(p, &x);
        println!("{}", **p);
    }
}

fn bar<'lt0, 'lt1>(p: &mut &'lt0 i32, x: &'lt1 i32)
    where
        'lt1: 'lt0,
{
    *p = &x;
}

/* first extraction
fn bar<'lt0, 'lt1, 'lt2>(p: &'lt0 mut &'lt1 i32, x: &'lt2 i32)
    where
        'lt2: 'lt1,
{
    *p = &x;
}
*/

const W: i32 = 5;

// does not work because improper ref handlings
pub fn in_out_lt() {
    let x = 1;
    let x_ref: &i32 = &x;
    let mut z: &i32;
    {
        let y = 2;
        z = &y;
        z = if z < x_ref { &&y } else { &W };

        println!("{}", *z);
    }
}

fn a() {
    let mut a = vec![1, 2, 3];
    let mut b = vec![5, 2, 3];
    let x = 1;
    let y = 2;

    println!("{}{}", x, y);
    a.push(4);
    let _ = a.get(0);
    b[0] = a[0];
    println!("{}{}", a[0], b[0]);
}

fn main() {
    lt_bounds();
    lt_bounds_use();
    in_out_lt();
    a()
}