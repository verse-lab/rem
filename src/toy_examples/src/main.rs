mod non_control_flow;
mod in_out_lifetimes;
mod lifetime_bounds;
mod extract_to_trait;
mod multiple_expressions_with_different_lifetimes;

fn main() {
    non_control_flow::original_foo();
    in_out_lifetimes::original_foo();
    lifetime_bounds::original_foo();
    multiple_expressions_with_different_lifetimes::original_foo();
}