mod borrow;

fn main() {
    borrow::make_borrows(
        "input/vec_borrow_mut.rs",
        "output/vec_borrow_mut.rs",
        "bar",
        "new_foo",
    )
}
