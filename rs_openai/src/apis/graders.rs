//! Grading logic used by Evals. There is no standalone `/graders` REST endpoint -- graders
//! only ever exist embedded within an eval's `testing_criteria` array (see
//! `crate::interfaces::evals::EvalGrader` and its 5 variants:
//! `StringCheck`/`TextSimilarity`/`Python`/`ScoreModel`/`LabelModel`). This accessor is kept
//! for API-surface completeness (matching Phase 2a's original scaffold) but intentionally has
//! no methods, since none exist on OpenAI's side to call -- see
//! docs/superpowers/specs/2026-08-14-evals-design.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct Graders<'a> {
    openai: &'a OpenAI,
}

impl<'a> Graders<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}
