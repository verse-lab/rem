use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Display;

use rem_utils::annotation::Annotated;

/// Abstract encoding of a Local Constraint
pub trait LocalConstraint: Any + Display + Clone {
    /// static CHR rules for the constraint system
    const CHR_RULES: &'static str;

    /// parse a single constraint rule
    fn parse(s: &str) -> nom::IResult<&str, Self>;

    /// Collect CHR rules from a function definition
    fn collect<'a>(fun: &Annotated<'a, &'a syn::ItemFn>) -> Vec<Self>;
}

trait LocalConstraintSystem {
    fn analyze<'a>(&mut self, fun: &Annotated<'a, &'a syn::ItemFn>);
    fn constraints(&self) -> Vec<Box<dyn Any>>;
}

struct ConstraintSystem<C: LocalConstraint> {
    constraints: Vec<C>,
}

impl<C: LocalConstraint> Default for ConstraintSystem<C> {
    fn default() -> Self {
        ConstraintSystem {
            constraints: vec![],
        }
    }
}

impl<C: LocalConstraint + 'static> LocalConstraintSystem for ConstraintSystem<C> {
    fn analyze<'a>(&mut self, fun: &Annotated<'a, &'a syn::ItemFn>) {
        self.constraints = C::collect(fun);
        // println!("collected");
        // for x in &self.constraints {
        //     println!("collected constraints: {}", x);
        // }
        self.constraints = crate::chr::chr_solve(&self.constraints);
    }

    fn constraints(&self) -> Vec<Box<dyn Any>> {
        let constraints: Vec<C> = self.constraints.clone();
        let constraints: Vec<Box<dyn Any>> = constraints
            .into_iter()
            .map(|v| {
                let boxed: Box<dyn Any> = Box::new(v);
                boxed
            })
            .collect::<Vec<_>>();
        constraints
    }
}

pub struct ConstraintManager {
    /// mapping of type ids to a name + constraint system
    constraint_systems: HashMap<TypeId, (&'static str, Box<dyn LocalConstraintSystem>)>,
}

impl Default for ConstraintManager {
    fn default() -> Self {
        ConstraintManager {
            constraint_systems: HashMap::new(),
        }
    }
}

impl Display for ConstraintManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConstraintManager(")?;
        for (_, (name, _)) in self.constraint_systems.iter() {
            write!(f, "{}, ", name)?;
        }
        write!(f, ")")?;
        Ok(())
    }
}

impl ConstraintManager {
    pub fn add_constraint<C: LocalConstraint>(&mut self) {
        let id = TypeId::of::<C>();
        let lcs = ConstraintSystem::<C>::default();
        let name = std::any::type_name::<C>();
        self.constraint_systems.insert(id, (name, Box::new(lcs)));
    }

    pub fn get_constraints<C: LocalConstraint>(&self) -> Vec<C> {
        let id = TypeId::of::<C>();
        let constraint_system = self.constraint_systems.get(&id);

        match constraint_system {
            Some((_, lcs)) => lcs
                .constraints()
                .into_iter()
                .map(|boxed| std::boxed::Box::into_inner(boxed.downcast::<C>().unwrap()))
                .collect(),
            None => vec![],
        }
    }

    pub fn analyze<'a>(&mut self, fun: &Annotated<'a, &'a syn::ItemFn>) {
        for (_k, (_, v)) in self.constraint_systems.iter_mut() {
            v.analyze(fun)
        }
    }
}
