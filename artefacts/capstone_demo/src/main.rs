fn main() {
    foo()
}
const W: [i32; 1] = [5];
pub fn foo() {
    let x = [1];
    let xref = &x;
    let mut z : &[i32];
    {
        let y = [2];
        z = &y;
        z = if z[0] < xref[0] {
            &y
        } else {
            &W
        };
        println!("{:?}", z);
    }
}