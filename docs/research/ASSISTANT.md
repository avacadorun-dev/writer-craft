# WriterCraft: assistant architecture and desktop design

Research date: 2026-11-07. Read-only inspection of Craft Anything at commit `05bd9e9e92a51dd771c68727c5e5e0e73fb29522`, plus the current Writers Studio checkout. This document is implementation guidance for a new native Rust MVP, not a claim that an early prototype has production parity.

## 1. The useful part of Craft Anything for our assistant

The strongest transferable idea is one local application engine shared by human UI actions and agent tools. Reverse-engineering capabilities are irrelevant to an author's application. Do not depend on `craft-anything-engine` or `craft-anything-ai` wholesale: their manifests and imports pull in formats, disassembly, JavaScript analysis, devices, builders and generated game assets.

Primary sources below use the inspected commit so statements remain reproducible:

| Source | Observed mechanism | Adaptation for Writers Studio |
| --- | --- | --- |
| [engine/lib.rs](https://github.com/storytold/craft-anything/blob/05bd9e9e92a51dd771c68727c5e5e0e73fb29522/crates/engine/src/lib.rs) | `Engine::call` checks availability and consent, runs a registered operation, journals the outcome and catches escaped panics. UI can try the session lock without blocking a frame. | A typed `AppCommand` dispatcher is the sole mutation path for UI, CLI and later MCP. Avoid a session-wide lock across network calls. |
| [engine/tool.rs](https://github.com/storytold/craft-anything/blob/05bd9e9e92a51dd771c68727c5e5e0e73fb29522/crates/engine/src/tool.rs) | Operations declare effects and requirements; toolsets allow gradual exposure. | Small tools grouped as project, manuscript, context, proposals. Declare read, propose and commit effects separately. Never copy its consent predicate as our manuscript-edit policy. |
| [evidence/lib.rs](https://github.com/storytold/craft-anything/blob/05bd9e9e92a51dd771c68727c5e5e0e73fb29522/crates/evidence/src/lib.rs) | Records include content-derived identity, provenance, coverage, limitations and locations. Confidence distinguishes observed, derived and inferred. | Cite manuscript node/revision/range or transcript/time range. Treat a quote as observed and literary interpretation as inferred. An AI assertion does not become a verified story fact just because it was recorded. |
| [investigation_tools.rs](https://github.com/storytold/craft-anything/blob/05bd9e9e92a51dd771c68727c5e5e0e73fb29522/crates/engine/src/tools/investigation_tools.rs) | Structured brief, plan, hypotheses, findings and handoff persist across turns. Revision checks and update locks reject stale writes. | Preserve task, selected scope, constraints, short plan, relevant facts and last handoff for each assistant thread. Keep manuscript changes separate from task-memory changes. |
| [ai/context.rs](https://github.com/storytold/craft-anything/blob/05bd9e9e92a51dd771c68727c5e5e0e73fb29522/crates/ai/src/context.rs) | Old tool output is trimmed before older conversation is summarized. Recent exchanges and provider-required call/result structure are preserved. | Scope context to selection/current document plus requested references. Persist full source separately so compacted context can retrieve exact text again. Do not copy fixed cloud token limits. |
| [ai/agent.rs](https://github.com/storytold/craft-anything/blob/05bd9e9e92a51dd771c68727c5e5e0e73fb29522/crates/ai/src/agent.rs) | Background loop emits events, compacts tool results, supports stop/cancel, turn budgets, permission requests and resumption. | Worker owns the provider request and sends bounded events. UI draws state only. Tool outcomes include source references and paging handles. Cancellation must stop future commits. |
| [engine/project.rs](https://github.com/storytold/craft-anything/blob/05bd9e9e92a51dd771c68727c5e5e0e73fb29522/crates/engine/src/project.rs) | Project directory carries versioned metadata and durable records. Files are written atomically; newer formats are refused. | Use a versioned project format and atomic persistence, plus explicit migration/backup. For an MVP, no silent overwrite after failing to parse an existing project. |
| [engine/journal.rs](https://github.com/storytold/craft-anything/blob/05bd9e9e92a51dd771c68727c5e5e0e73fb29522/crates/engine/src/journal.rs) | Append-only daily JSONL records, bounded fields and credential redaction. Logging is best effort. | Keep a compact operation journal for recovery/diagnostics, but use a transaction or atomic snapshot for actual edit durability. Best-effort logs alone cannot prove that an edit was committed. |
| [ai/keys.rs](https://github.com/storytold/craft-anything/blob/05bd9e9e92a51dd771c68727c5e5e0e73fb29522/crates/ai/src/keys.rs) | OS keychain first, labelled owner-only fallback file, environment fallback. Keychain access runs with a deadline. | Keys stay outside projects, proposals and exports. For MVP prefer session/env configuration until proper keychain integration exists. Do not silently persist API credentials in plain project JSON. |

Craft Anything's [README](https://github.com/storytold/craft-anything/blob/05bd9e9e92a51dd771c68727c5e5e0e73fb29522/README.md) explicitly separates internal analysis from a coding agent installed on the user's machine and describes limitations of guardrails. Our writing assistant should call its own tightly scoped manuscript tools. It does not need a general coding agent, shell or arbitrary filesystem access.

### Reuse boundary

- Reimplement the small architectural patterns against our domain. No need to vendor their engine.
- Evidence is the smallest potentially extractable crate conceptually. A handful of project-specific provenance types is probably clearer than the generic reverse-engineering schema.
- Provider streaming and context logic are useful references, but have provider-specific edge cases. Adopt only after isolating them behind our own `AssistantProvider` interface and checking their license/attribution requirements if code is copied.
- The manuscript review/apply/undo transaction described below is **our design**, informed by the existing Writers Studio guarantees. I did not find a generic manuscript proposal transaction in the inspected Craft Anything engine. Do not claim that importing its consent UI implements edit review.

## 2. MVP assistant contract

Minimum useful local tools: read current selection, read node at revision, search project, list structure, retrieve dictionary/context, propose replacement, list proposals. The assistant gets no direct `write_text` primitive. Human apply/undo commands share the application dispatcher, but are not automatically exposed as model tools.

Each request captures `project_id`, `node_id`, `base_revision`, selected text and explicit user intent. Each proposal stores those IDs, exact expected text, replacement text, explanation, evidence references and creation time. For a block editor prefer stable block IDs plus offsets; a plain-text MVP may use a character range with a revision and exact-text check. Byte offsets must not be confused with character positions in Cyrillic/Unicode text.

State machine: `generating → proposed → applied | rejected | stale | cancelled`. Reopening the application preserves proposed and applied states. No provider response overwrites the manuscript. Applying validates revision and expected text, commits document revision and proposal status together, then records the operation. A stale proposal prompts regeneration or explicit rebase/review. Undo verifies the post-apply revision/text; it must not replace later user edits blindly.

User-facing labels:

- Panel tabs: `Ассистент`, `Заметки`, `Версии`.
- Scope control: `Выделение` / `Документ` / `Проект`; default to selection when available, otherwise current document. Show selected scope before sending.
- Pending proposal header: `Предлагаемые изменения · не применены`.
- Comparison labels: `Исходный текст` and `Предложение`. Include document name, count of replacements and a short reason.
- Primary action: `Применить изменения`. Secondary: `Отклонить`. Applied state: `Изменения применены`, with `Отменить применение`.
- Stale state: `Документ изменился. Обновите предложение перед применением.`
- Analysis-only answer: `Замечания по тексту`; no Apply button unless there is a real edit proposal.
- Offline/no-provider mode: `Локальная проверка` for deterministic checks. Never present canned local suggestions as output of an LLM. If no provider is configured, show a working provider-setup action.
- Cancelled request: retain readable partial answer but disable applying incomplete proposals.

Provider calls, import/export work and long searches run off the egui thread. Bounded event channels carry `Started`, `TextDelta`, `ProposalReady`, `Completed`, `Failed`, `Cancelled`. UI must handle late events from an older request ID safely. A hidden inspector does not kill a request; Stop does. Repeated Send does not silently run multiple competing edits.

## 3. Native MVP implementation boundaries

The initial desktop uses the native egui 0.36.2 UI. The application keeps domain commands and persistence outside the renderer. Writing is available offline; network access is needed only for an explicitly configured remote provider. A local OpenAI-compatible endpoint may also be configured, but bundling a local model is a separate distribution decision.

This document describes target assistant behavior. Implemented capability must be verified against code and tests before being advertised: deterministic local diagnostics are not an AI provider, a saved proposal is not an applied edit, and storing a provider URL does not prove that streaming or tool calls work.

For an MVP with whole-document proposals, capture the entire source document and revision and display that scope honestly. Do not imply a selected-range operation if the provider receives or replaces the whole document. Proposal Apply and Undo are core commands. Keep provider I/O on workers and show actionable errors.

Design tokens, pane layout, command behavior, icons and screenshots are specified in `../design/DESKTOP_UX.md`. The newer charcoal/blue reference supersedes the earlier warm light-theme exploration.

## 4. Acceptance checks

- Real configured provider: prompt → answer/proposal → review → Apply → Undo. No canonical changes before Apply.
- Stale source, Unicode ranges, cancellation and late request events cannot corrupt newer writing.
- Reopen a project with a pending proposal and an applied revision; status and revision guards remain consistent.
- No-provider and network-error flows preserve writing and identify the next useful action.
- Journal, exported projects, exception text and screenshots must not reveal credentials.
- Context scope is visible and evidence references refer to stable project/node/revision locations.
- No general shell execution, broad filesystem access or reverse-engineering dependencies are needed for manuscript assistance.
