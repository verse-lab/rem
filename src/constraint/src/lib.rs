#![feature(box_patterns)]
#![feature(box_into_inner)]

mod chr;
pub mod common;
mod constraint;

pub use constraint::ConstraintManager;
pub use constraint::LocalConstraint;
