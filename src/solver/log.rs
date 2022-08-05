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
        #[cfg(debug_assertions)]
        for cls in self.clause_db.iter() {
            let cls_str = self.trail.fmt_clause(&cls);
            debug!("{cls_str}");
        }
    }

    pub(crate) fn implication_graph_to_dot(
        &self,
        conflict: Option<Clause>,
        mut out: impl std::io::Write,
    ) -> Result<(), std::io::Error> {
        writeln!(out, "digraph {{")?;
        for elem in self.trail.trail() {
            let annotation = match elem.reason {
                TrailReason::Decision => "D",
                TrailReason::Propagated { .. } => "P",
                TrailReason::Axiom => "A",
            };

            writeln!(
                out,
                "{} [label = \"{}{annotation}\"];",
                elem.lit.var(),
                elem.lit
            )?;

            if let TrailReason::Propagated { cls } = elem.reason {
                let cls = self.clause_db.get(cls);

                for l in cls {
                    if l == elem.lit {
                        continue;
                    }

                    writeln!(out, "{} -> {};", l.var(), elem.lit.var())?;
                }
            }
        }

        if let Some(conflict) = conflict {
            writeln!(out, "X;");
            for l in conflict {
                writeln!(out, "{} -> X;", l.var())?;
            }
        }

        writeln!(out, "}}")
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

    pub(crate) fn fmt_clause(&self, clause: &[Lit]) -> String {
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
