mod non_control_flow;
mod in_out_lifetimes;
mod lifetime_bounds;
mod extract_to_trait;
mod multiple_expressions_with_different_lifetimes;

fn main() {
    non_control_flow::original_foo();
    non_control_flow::new_foo_fixed();

    in_out_lifetimes::original_foo();
    in_out_lifetimes::new_foo_fixed();

    lifetime_bounds::original_foo();
    lifetime_bounds::new_foo_fixed();

    {
        let x1 = &mut &0;
        let x2 = &mut &0;
        let n = &1;
        {
            let y1 = &1;
            let y2 = &1;
            let m = &0;
            multiple_expressions_with_different_lifetimes::original_foo1(x1, y1);
            multiple_expressions_with_different_lifetimes::original_foo2(x2, y2);
            multiple_expressions_with_different_lifetimes::new_foo1(x2, y2);

            extract_to_trait::original_foo(m, n);
            extract_to_trait::new_foo_fixed(m, n);
        }
    }

}