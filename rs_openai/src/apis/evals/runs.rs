//! An eval's runs, including `POST .../runs/{run_id}/cancel`. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct EvalRuns<'a> {
    openai: &'a OpenAI,
    eval_id: String,
}

impl<'a> EvalRuns<'a> {
    pub(crate) fn new(openai: &'a OpenAI, eval_id: String) -> Self {
        Self { openai, eval_id }
    }
}

#[allow(dead_code)]
pub struct EvalRun<'a> {
    openai: &'a OpenAI,
    eval_id: String,
    run_id: String,
}

impl<'a> EvalRun<'a> {
    pub(crate) fn new(openai: &'a OpenAI, eval_id: String, run_id: String) -> Self {
        Self {
            openai,
            eval_id,
            run_id,
        }
    }
}
