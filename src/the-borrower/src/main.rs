mod borrow;

fn main() {
    borrow::make_borrows(
        "input/borrow_write_use_after.rs",
        "output/borrow_write_use_after.rs",
        "bar",
        "new_foo",
    )
}
