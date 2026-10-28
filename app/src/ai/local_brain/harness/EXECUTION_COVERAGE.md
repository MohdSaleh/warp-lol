# Agent Harness — Execution Flow & Integration Coverage

Status of the local agent harness (`app/src/ai/local_brain/harness/`). This file is
the single list of what the harness must cover, how it maps to established
open-source harness architecture, and where each feature stands.

Status legend: **Done** (implemented + tested) · **Next** (current phase) ·
**Planned** (scheduled) · **Controller** (intentionally stays in the app client).

## Reference architecture

The loop follows the shape shared by the **Claude Agent SDK** and the
**OpenAI Agents SDK** — the two mainstream open-source harness references:

| Harness concept | Reference | Local implementation |
| --- | --- | --- |
| Agent loop (run until no tool calls) | Claude Agent SDK `query()` loop; OpenAI Agents SDK Runner | `AgentHarness` phase machine (`phase.rs`) |
| Function tools | Both: tools + JSON-schema function calling | `HarnessToolExecutor` / `ToolRegistry` (`executor.rs`) |
| Tool guardrails | OpenAI tool guardrails; Claude PreToolUse hooks | `ToolOutcome::Rejected` + policy gate (Planned) |
| Permissions | Claude permission modes (ask/allow/deny) | Tool allowlist per run (Planned) |
| Hooks (pre/post tool) | Claude Agent SDK hooks | `HarnessHook` trait (Planned) |
| Sessions & resume | Claude sessions; OpenAI Sessions | Ledger + durable run state (Planned) |
| Subagents | Claude subagents (isolated context/tools) | `run_agents` → local child harness runs (Planned) |
| Context compaction | Claude session compaction | `context::compaction` (already in the loop) |
| Retries / backoff | Both | `repair::systematic`, `repair::rate_limit` (already in the loop) |
| Tracing | OpenAI tracing | `persist_rich_events` (telemetry stays policy-gated) |
| Streaming output | Both: incremental deltas | `HarnessEvent::AssistantDelta` |

## Execution flow

```
User request
   │
   ▼
[Intake]            route to custom agent, select model, load ledger
   │
   ▼
[ContextAssembly]   rebuild transcript, project rules, compaction, tool schemas
   │
   ▼
[ModelTurn] ───────► stream assistant deltas ──► no tool calls ──┐
   │                                                             │
   │ tool calls                                                  │
   ▼                                                             │
[ToolBatch]         run harness-owned calls; defer client calls    │
   │                                                             │
   ▼                                                             │
[Observation]       append results + outcomes to the transcript  │
   │                                                             │
   └────────────► back to [ModelTurn] (budget-gated)             │
                                                                 ▼
                                                          [Finalizing]
                                                                 │
                                                ┌────────────────┼────────────────┐
                                                ▼                ▼                ▼
                                            Completed          Failed         Cancelled
```

Invariants enforced by the harness, not by the caller:

- Every phase transition is validated (`InvalidTransition` on illegal moves).
- Budgets are checked before each model turn (6 turns / 12 tool calls / 6
  no-progress batches, from `LoopControlProfile::contract()`).
- A tool batch with no successful outcome increments the no-progress streak;
  any success resets it.
- Terminal phases absorb: a finished run cannot be revived.

## Feature coverage

| Feature | Phase | Status |
| --- | --- | --- |
| Phase state machine + illegal-transition rejection | all | **Done** |
| Single outbound event contract (`HarnessEvent`) | all | **Done** |
| Tool executor abstraction + registry dispatch | ToolBatch | **Done** |
| Filesystem executors: `read_files`, `glob`, `grep` | ToolBatch | **Done** |
| `read_files` paging (`offset`/`limit`) + line-numbered output | ToolBatch | **Done** |
| Oversized observations spilled to a retained local file | ToolBatch | **Done** |
| Bounded concurrency for read-only tool batches | ToolBatch | **Done** |
| Budget wrap-up turn (text-only final answer, tools disabled) | ModelTurn | **Done** |
| Budget gate + progress accounting over the ledger | ModelTurn | **Done** |
| Execution ownership (`Controller` vs `Harness`) so tools never run twice | ToolBatch | **Done** |
| Harness wired into `generate_output` (turn-boundary budget, phases, wrap-up, ledger) | all | **Done** |
| Tool guardrails / permission allowlist per run | ToolBatch | Planned |
| Pre/post tool hooks | ToolBatch | Planned |
| Shell execution via `Session::execute_command` | ToolBatch | Planned (P2) |
| MCP tool adapter | ToolBatch | Planned (P2) |
| Policy-gated `web_search` / `web_fetch` | ToolBatch | Planned (P2) |
| Durable run state + resume after restart | all | Planned (P3) |
| Local subagents (isolated child harness runs) | ToolBatch | Planned (P3) |
| Graph-driven node policies (`GraphEngine.transition`) | Intake/ModelTurn | Planned (P3) |
| LocalOnly zero-egress enforcement at the boundary | all | Planned (P3) |
| Usage accounting (suppressed when LocalOnly) | Finalizing | Planned (P3) |

Already provided by the surrounding loop and retained: context compaction,
systematic/rate-limit/context-limit auto-recall, self-reflection injection,
rich-event persistence, live token streaming, cancellation.

### Wiring status

`generate_output` now owns the full turn loop through `AgentHarness`:
`ContextAssembly → ModelTurn → ToolBatch → Observation → ModelTurn …`, with the
budget gate (`begin_turn_with_repair`) re-checked before every model turn and
every exit path finishing the run and persisting the ledger.

Execution ownership is split per call. When a local session has a working
directory and is not remote, `generate_output` builds
`executors::filesystem_registry()` and classifies each turn's calls:

- **Harness-owned** (`read_files`, `glob`, `grep`; plus `update_todos`, which
  becomes an `UpdateTodos` message and a synthetic success result) run inside
  the request through `ToolRegistry::execute_batch`, respecting the per-turn
  call cap.
- **Client-owned** calls stay in the accumulator and leave through the existing
  action path exactly once.

If a turn contains only harness-owned calls, the loop keeps going without
returning to the client: results are appended to the in-memory transcript and
re-entered as replay messages — a `ToolCall` carrying `tool::Server` (opaque
`{"version","name","arguments"}` payload) paired with a
`tool_call_result::Result::Server`. The client renders neither and executes
neither, so nothing runs twice; `tools::server_tool` decodes these payloads in
history rendering and fingerprinting. A mixed turn executes its harness-owned
calls, then finalizes terminally so the client-owned calls go out once; a
client-only turn behaves exactly as before. In-process turns count against the
model-turn budget with per-turn request ids, so the wrap-up turn still fires
and repeated-call / no-progress guards see every call across requests.

## Borrowed from OpenCode (frictionless subset)

Only mechanisms that require no new prompt, dialog, or step from the user are
adopted. Everything that would add friction is listed as not adopted.

| OpenCode mechanism | Local implementation | Status |
| --- | --- | --- |
| `read` paging + `"<n>: line"` output + truncation footer | `executors/fs.rs` `render_read` | **Done** |
| `ToolOutput` spill: full output kept past the inline limit | `output.rs` + `ToolRegistry::execute` | **Done** |
| Tools forked as calls stream, bounded concurrency | `ToolRegistry::execute_batch` (read-only only) | **Done** |
| `MAX_STEPS_PROMPT` + tools disabled on the final step | `AgentHarness::begin_turn` → `TurnDirective::WrapUp` → `generate_output` wrap-up turn | **Done** |
| Permission rules / default `ask` | Not adopted: would add approval clicks | — |
| Preview-diff approval for edits | Not adopted yet: no write executor in the harness | P2 |
| Snapshot store + revert | Not adopted yet: write tools land in P2 | P2 |
| Inbox steer/queue, run claims, subagent sessions | Not adopted: single in-process run, no UI benefit | — |

## Tool coverage

`Harness` = executed inside the loop; `Controller` = stays in the app client
(UI/backend-bound); `Next`/`Planned` = executor scheduled.

| Tool | Owner | Status |
| --- | --- | --- |
| `read_files` | Harness | **Done** (paged, line-numbered) |
| `glob` | Harness | **Done** |
| `grep` | Harness | **Done** (now returns matched line text) |
| `apply_file_diffs` | Harness | Next (reuse `diff_application::apply_edits`) |
| `read_skill` | Harness | Planned (P2, filesystem) |
| `update_todos` | Harness | **Done** (structural: `UpdateTodos` message + synthetic result) |
| `yield_state` | Harness | Planned (P2, run state) |
| `run_command` | Harness | Planned (P2, needs `Session` handle) |
| `read_command_output` | Harness | Planned (P2) |
| `write_to_command` | Harness | Planned (P2) |
| `web_search` | Harness | Planned (P2, policy-gated) |
| `web_fetch` | Harness | Planned (P2, policy-gated) |
| `mcp__*` | Harness | Planned (P2, MCP bridge) |
| `read_mcp_resource` | Harness | Planned (P2) |
| `search_codebase` | Harness | Planned (P3, local index) |
| `ask_user_question` | Hybrid | Planned (P3, UI bridge) |
| `run_agents` | Harness | Planned (P3, child runs) |
| `subagent` | Harness | Planned (P3) |
| `send_message_to_agent` | Harness | Planned (P3) |
| `wait_for_events` | Harness | Planned (P3) |
| `use_computer` | Controller | GUI-bound (never in-process) |
| `request_computer_use` | Controller | GUI-bound |
| `open_code_review` | Controller | UI-bound |
| `insert_review_comments` | Controller | UI-bound |
| `init_project` | Controller | UI-bound |
| `upload_file_artifact` | Controller | Backend-bound |
| `fetch_conversation` | Controller | Backend-bound |
| `read_documents` | Controller | Document viewer state |
| `create_documents` | Controller | Document viewer state |
| `edit_documents` | Controller | Document viewer state |
| `suggest_prompt` | Controller | Output-only |
| `suggest_new_conversation` | Controller | Output-only |

## Verification

| Area | Test |
| --- | --- |
| Phase machine | `harness::phase::tests` |
| Executor registry + batch concurrency | `harness::executor::tests` |
| Harness state / budgets / wrap-up | `harness::tests` |
| Filesystem executors | `harness::executors::fs::tests` |
| Observation bounding / spill | `harness::output::tests` |
| Turn-boundary wiring, wrap-up turn, tool suppression | `streaming::tests::wrap_up_turn` |
| In-process execution, ownership split, no double execution, budget continuity | `streaming::tests::in_process_tools` |

Run: `CARGO_INCREMENTAL=0 cargo test -j 2 -p warp --lib local_brain::harness`
