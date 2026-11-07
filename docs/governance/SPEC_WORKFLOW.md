# Spec-driven engineering workflow

WriterCraft uses a small repository-local adaptation of [Fission-AI OpenSpec](https://github.com/Fission-AI/OpenSpec). Its official [spec-driven schema](https://github.com/Fission-AI/OpenSpec/blob/main/schemas/spec-driven/schema.yaml) separates proposal, capability deltas, technical design and tasks; [project configuration](https://github.com/Fission-AI/OpenSpec/blob/main/docs/customization.md) supplies context and artifact rules. These conventions were checked on 2026-11-07. We are not creating a second product framework or installing a runtime dependency into the desktop app.

## Sources of truth

| Question | Authoritative file |
|---|---|
| Why and for whom? | `docs/PRD.md` |
| What observable behavior is required? | `openspec/specs/<capability>/spec.md` |
| What is changing now? | `openspec/changes/<id>/proposal.md` and change-local `specs/` deltas |
| How and why this architecture? | Change `design.md` and `docs/architecture/DECISIONS.md` |
| What remains and what passed? | Change `tasks.md`, linked named tests and verification artifacts |
| What is still missing from legacy product? | `docs/migration/FEATURE_PARITY.md` |

The initial canonical specs are a bootstrap target contract. Some explicitly mark a later parity milestone. A requirement's presence does not mean implementation or acceptance is complete. `native-mvp` contains only the MVP deltas and remains open while delivery checks are incomplete.

## Agent loop

1. Read root `AGENTS.md`, PRD, relevant capability specs, accepted ADRs, the active change and applicable Rust skill chapters. Inspect actual source before assuming a feature exists.
2. For a behavior change, update its proposal and precise capability delta before implementation. Use one durable capability name, `### Requirement:` blocks and `#### Scenario:` WHEN/THEN cases. Put internals in design rather than product requirements.
3. Keep each task bounded and connect it to an observable scenario. Implement within the user's existing authorization; the workflow does not require repeated permission for already-authorized changes. Ask only when a missing decision materially changes scope or irreversible action needs authorization.
4. Run focused checks first, then required formatting/tests/Clippy. Exercise native UI changes visually. Preserve user work on errors; record material limitations and hypotheses explicitly.
5. Check off a task only after its stated evidence exists. Update parity and handoff records when scope changes. A compiler pass cannot establish UX, accessibility, migration parity or provider behavior.
6. On completion, verify the delta against current specs, merge the accepted behavior contract and archive the change with dated evidence. Do not archive an incomplete milestone to make the checklist look finished.

## OpenSpec integration status

The `openspec/` config, canonical specs and native-mvp change were authored against official conventions. Official OpenSpec1.16.1 strict CLI validation passed for all four canonical specs and the native-mvp change; see [validation evidence](OPENSPEC_VALIDATION.md). `openspec init` and generated agent integrations were not run. Project instructions work by reading these files even before optional CLI integration.

When a pinned CLI is available, the official [CLI reference](https://github.com/Fission-AI/OpenSpec/blob/main/docs/cli.md) documents `openspec list --specs`, `openspec status --change native-mvp`, and `openspec validate native-mvp --strict`. Capture the actual version, command and result before marking validation complete. Keep generated tool files separate from hand-maintained repository instructions. Installing or updating tooling must not rewrite project rules implicitly.

## Minimal acceptance evidence

Every completed change names the relevant spec IDs, actual test names/commands, visible UI evidence if applicable, limitations and remaining risks. The native core currently has twenty-four acceptance tests; the MVP task document maps all twenty-four by their real function names. Theme and rich-parity scenarios stay pending until implemented and observed.
