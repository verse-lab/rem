mod borrow;

fn main() {
    borrow::make_borrows("input/borrow.rs", "output/borrow.rs", "extract_read_use_after_bar", "extract_read_use_after_new")
}
