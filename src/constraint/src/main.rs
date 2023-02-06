#![feature(box_patterns)]
use constraint::{
    common::{MutabilityConstraint},
    ConstraintManager,
};



fn main() {
    let ast: syn::ItemFn = syn::parse_str(
        "
fn new_foo() {
    let mut x = vec![];
    x.get(1);
    if x[0] > 1 {
        println!(\"something\")
    }
}
",
    )
    .unwrap();
    let annot_ast = utils::annotation::annotate_ast(&ast);
    let mut cs = ConstraintManager::default();
    //cs.add_constraint::<ArrayConstraint>();
    cs.add_constraint::<MutabilityConstraint>();

    cs.analyze(&annot_ast);

    for constraint in cs.get_constraints::<MutabilityConstraint>().iter() {
        println!("{}", constraint);
    }
}
