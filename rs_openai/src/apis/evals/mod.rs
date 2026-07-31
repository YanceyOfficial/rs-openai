//! Evaluation runs for grading model outputs against a defined criteria set. Not yet
//! implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

pub mod runs;

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct Evals<'a> {
    openai: &'a OpenAI,
}

impl<'a> Evals<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

pub struct Eval<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Eval<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for this eval's runs. Not yet implemented.
    pub fn runs(&self) -> runs::EvalRuns<'a> {
        runs::EvalRuns::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for a specific run of this eval. Not yet implemented.
    pub fn run(&self, run_id: impl Into<String>) -> runs::EvalRun<'a> {
        runs::EvalRun::new(self.openai, self.id.clone(), run_id.into())
    }
}
