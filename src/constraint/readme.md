# CHR-based Constraint Systems
This sub-module implements a collection of CHR-based constraint
systems, and a generic framework for abstracting Rust ASTs using
constraints.


## Overview
The end user interface to this module is captured through two main elements:

- *LocalConstraint* - a trait that your custom constraint should implement
   ```rust
   /// Abstract encoding of a Local Constraint
   pub trait LocalConstraint : Any + Display + Clone {
   
       /// static CHR rules for the constraint system
       const CHR_RULES: &'static str;
   
   
       /// parse a single constraint rule
       fn parse(s: &str) -> nom::IResult<&str, Self>;
   
       /// Collect CHR rules from a function definition
       fn collect<'a>(fun: &Annotated<'a, syn::ItemFn>) -> Vec<Self>;
   }
   ```
- *ConstraintManager* - used to represent a collection of constraint systems, encapsulating the process of running each individual constraint
   ```rust
   let mut constraint_manager : ConstraintManager = Default::default();
   constraint_manager.add_constraint::<ArrayConstraint>();
   constraint_manager.add_constraint::<MutabilityConstraint>();

   // ...
   constraint_manager.analyze(fun);
   
   // ..
   let arr_constraints : Vec<ArrayConstraint> = constraint_manager.get_constraints::<ArrayConstraint>();
   let mut_constraints : Vec<MutabilityConstraint> = constraint_manager.get_constraints::<MutabilityConstraint>();
   ```

## Project structure

```
./src/
├── constraint.rs
├── annotation.rs
├── chr.rs
├── common.rs
└── lib.rs

1 directory, 10 files
```

The sub-component is composed of the following modules:

- constraint.rs -- defines the core generic constraint collection framework
- annotation.rs -- defines a visitor to label the syn ASTs
- chr.rs -- defines a wrapper around SwiPL to run CHR rules over a list of constraints
- common.rs -- defines a few instantiations of the constraint system for mutability and arrays



Parser should be moved to utils, and constraint specific parts moved into common
Solver should be moved to rewrite
