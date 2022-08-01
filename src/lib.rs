#![feature(nonzero_ops)]
#![feature(type_alias_impl_trait)]
#![feature(iter_intersperse)]

mod dimacs;
mod solver;
mod util;

pub use solver::{Model, Proof, Result, Solver};
