#![feature(box_patterns)]

use std::collections::HashMap;
use constraint::{
    common::{AliasConstraints},
    ConstraintManager,
};
use itertools::Itertools;


fn main() {
    let ast: syn::ItemFn = syn::parse_str("
    pub fn new_foo() {
    let W: i32 = 5;
    let x = 1;
    let x_ref = &x;
    let m = x_ref;
    let mut z: &i32;
    {
        let y = 2;
        z = &y;
        z = if *z < *x_ref {
        m
    } else {
        &W
    };
        println!(\"{}\", *z);
    }
}")
    .unwrap();
    let mut lookup = HashMap::new();
    let mut cs = ConstraintManager::default();

    let annot_ast = utils::annotation::annotate_ast(&ast, &mut lookup);

    //cs.add_constraint::<ArrayConstraint>();
    cs.add_constraint::<AliasConstraints>();

    cs.analyze(&annot_ast);
    let constraints = cs.get_constraints::<AliasConstraints>();
    let constraints : Vec<AliasConstraints> = constraints.into_iter().unique().collect();

    for constraint in constraints {
        println!("{}", constraint);
        // match constraint {
        //     AliasConstraints::Ref(l) => println!("{} -> {:?}", l, lookup.get(&l)),
        //     AliasConstraints::Alias(l, r) => println!("aliased {} -> {:?}, {} -> {:?}", l, lookup.get(&l), r, lookup.get(&r)),
        //     AliasConstraints::Assign(l, r) => println!("assigned {} -> {:?}, {} -> {:?}", l, lookup.get(&l), r, lookup.get(&r)),
        // }
    }
}
