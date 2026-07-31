# Field diff: Assistants / Threads / Messages / Runs (low-priority — sunsetting)

Deliberately brief. The Assistants API sunsets **2026-08-26** (per OpenAI's deprecations page, confirmed during Phase 1 planning) and the plan's direction is to deprecate `assistants`/`threads`/`messages`/`runs` in place rather than invest further in them — this note exists to record the two concrete gaps already suspected from the structure-tree research, not to spec a full upgrade.

Source: `https://developers.openai.com/api/reference/llms.txt` and the two endpoint pages cited below.

## Runs — confirmed gap: Submit Tool Outputs

`rs_openai/src/interfaces/runs.rs` has `CreateRunRequest`, `ModifyRunRequest`, `DeleteRunResponse`, `RunResponse` — no equivalent of OpenAI's "submit tool outputs to a run" operation.

- `POST /v1/threads/{thread_id}/runs/{run_id}/submit_tool_outputs` — source: `https://developers.openai.com/api/reference/resources/beta/subresources/threads/subresources/runs/methods/submit_tool_outputs.md`
- Purpose: when a run enters `requires_action` status because the assistant wants to call a function, this is how the caller returns the function's actual output so the run can continue. Without this endpoint, **any run that uses function calling can never complete** — it will sit in `requires_action` forever from this crate's perspective.
- Request: `tool_outputs` (array of `{tool_call_id, output}`), optional `stream`.
- Response: a `Run` object (same shape as existing `RunResponse` should have — status, usage, `required_action` if more tool calls are needed).

## Runs — confirmed gap: Run Steps sub-resource

No "run steps" concept exists anywhere in the crate today.

- `GET /v1/threads/{thread_id}/runs/{run_id}/steps` (list), `GET /v1/threads/{thread_id}/runs/{run_id}/steps/{step_id}` (retrieve) — source: `https://developers.openai.com/api/reference/resources/beta/subresources/threads/subresources/runs/subresources/steps.md`
- Purpose: inspecting the individual actions (message creation, code interpreter call, file search call, function call) a run took internally — useful for debugging/observability, not required for basic run completion the way Submit Tool Outputs is.
- Key fields: `id`/`run_id`/`assistant_id`/`thread_id`, timing (`created_at`/`completed_at`/`failed_at`/`cancelled_at`/`expired_at`), `status`, `step_details` (message-creation or tool-call variant), `last_error`, token `usage`, `metadata`.

## Assistants, Threads, Messages

No additional gaps beyond what the structure-tree research already recorded (operation sets for these three otherwise match OpenAI's current reference). Not investigated further given the sunset.

## Recommendation (unchanged from Phase 1 planning)

Don't fill either gap above. Deprecate `assistants`/`threads`/`messages`/`runs` in place, point users at the Responses API + Conversations API as the replacement, matching OpenAI's own migration guidance. Sunset: 2026-08-26.
