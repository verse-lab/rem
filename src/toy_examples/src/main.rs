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
        let x = &mut &0;
        {
            let y = &1;
            multiple_expressions_with_different_lifetimes::original_foo2(x, y);
            multiple_expressions_with_different_lifetimes::new_foo1(x, y);
        }
    }

}