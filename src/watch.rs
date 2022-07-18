use crate::{clause::ClauseIdx, data::LitVec, Lit};

#[derive(Clone)]
pub struct Watch {
    pub clause: ClauseIdx,
}
