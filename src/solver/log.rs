use tracing::{debug, instrument};

use super::{
    clause::Clause,
    trail::{Trail, TrailReason},
    Lit, Solver,
};

const RED: &str = "\u{1b}[31m";
const GREEN: &str = "\u{1b}[32m";
const END: &str = "\u{1b}[0m";

#[allow(unused)]
impl Solver {
    #[instrument(skip_all)]
    pub(crate) fn log_state(&self) {
        for cls in self.clause_db.iter() {
            let cls_str = self.trail.fmt_clause(cls);
            debug!("{cls_str}");
        }
    }
}

impl Trail {
    pub(crate) fn fmt_lit(&self, lit: Lit) -> String {
        match self.get_lit_assignment(lit) {
            Some(true) => format!("{GREEN}{}{END}", lit.get()),
            Some(false) => format!("{RED}{}{END}", lit.get()),
            None => format!("{}", lit.get()),
        }
    }

    pub(crate) fn fmt_clause(&self, clause: Clause) -> String {
        clause
            .iter()
            .map(|&lit| self.fmt_lit(lit))
            .intersperse(", ".into())
            .collect::<String>()
    }

    pub(crate) fn fmt_trail(&self) -> String {
        let lst = self
            .trail()
            .iter()
            .map(|trail_elem| {
                let lit = trail_elem.lit;
                match trail_elem.reason {
                    TrailReason::Decision => format!("{lit}D"),
                    TrailReason::Propagated { .. } => format!("{lit}P"),
                    TrailReason::Axiom => format!("{lit}A"),
                }
            })
            .intersperse(", ".to_string());

        let mut result = "[".to_string();

        result.extend(lst);
        result.push(']');
        result
    }
}
