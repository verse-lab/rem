#![feature(box_patterns)]
use constraint::{
    common::{ArrayConstraint, MutabilityConstraint},
    ConstraintManager,
};

use syn::visit::Visit;

fn main() {
    let ast: syn::ItemFn = syn::parse_str(
        "
pub unsafe extern \"C\" fn reset(mut arr: *mut libc::c_int,
                               mut size: libc::c_int) {
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < size { *arr.offset(i as isize) = 0 as libc::c_int; i += 1 };
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
