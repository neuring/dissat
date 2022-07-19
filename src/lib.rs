#![feature(nonzero_ops)]
#![feature(iter_intersperse)]

mod dimacs;
mod solver;
mod util;

pub use solver::{Model, Proof, Result, Solver};
