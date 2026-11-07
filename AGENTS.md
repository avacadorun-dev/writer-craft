# Working agreements

- Use Rust for application code. No WebView, Electron or browser UI.
- Never create branches prefixed `codex/`; use `feature/`, `fix/`, `chore/`.
- Read `docs/architecture/`, `docs/design/` and relevant migration contracts before changing behavior.
- Follow `.agents/skills/rust-best-practices/SKILL.md` and relevant reference chapters for Rust work.
- Keep domain and persistence independent of desktop UI. GUI and headless tools share core commands.
- Preserve user text, source provenance, revisions, annotations, audio and unknown imported rich data.
- A proposal never changes canonical content until explicit Apply. Guard Apply and Undo against stale revisions.
- Use Unicode scalar positions in exact text anchors; never mix byte and UTF-16 offsets.
- Errors must be actionable. No production unwrap/expect, silent save failures or invented success.
- Never present scripted suggestions as AI output. Do not claim feature parity before migration acceptance passes.
- One desktop window: shared command bar, bounded panes, local scrolling, no nested app frames or browser controls.
- Icons use one coherent vector system, with tooltips and labels; no emoji as UI icons.
- Run cargo fmt, tests and clippy. Verify native app visually after UI changes.
- Commit focused, working increments. Update handoff and verification evidence when work stops.
- Do not modify the original Writers Studio repository or user projects without a separate instruction.
