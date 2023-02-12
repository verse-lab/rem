#![feature(box_patterns)]
use constraint::{
    common::{AliasConstraints},
    ConstraintManager,
};


fn main() {
    let ast: syn::ItemFn = syn::parse_str(
        "
fn new_foo() {
    let mut x = 0;
    let z = &x;
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
    cs.add_constraint::<AliasConstraints>();

    cs.analyze(&annot_ast);

    for constraint in cs.get_constraints::<AliasConstraints>().iter() {
        println!("{}", constraint);
    }
}
