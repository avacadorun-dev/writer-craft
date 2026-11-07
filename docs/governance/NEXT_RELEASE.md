# Next release: discussion brief

Status: unscheduled planning, not an implementation instruction. Updated 2026-11-08. Finish the separately approved first-alpha scope, then collect the owner's findings and choose the next release together. No next version, milestone date or persistent-annotation implementation is selected by this document.

## Candidate paths

| Path | Candidate scope | Entry evidence | Decision still needed |
|---|---|---|---|
| Focused bug-fix alpha | Verified first-alpha defects and narrowly bounded interaction/distribution regressions. `alpha.2` is only an illustrative candidate if it remains the same intended release. | Owner issue list, reproducible steps, expected/actual behavior, affected artifacts and regression coverage. | Which issues block the next build, acceptance scope and exact version. |
| Rich-model parity milestone | A reviewed foundation for legacy structured documents and durable text-note anchors, sized against the source inventory. | Existing rich schema/annotation contracts, migration fixtures, a native editor/anchor feasibility result and explicit selected workflows. | Whether this broader milestone is next, its boundaries, prerequisites and version. |

These are alternatives for discussion, not two automatically scheduled releases. Existing Writers Studio parity remains the product direction. New unrelated domain features are outside this brief. Portable engineering previews remain distinct from complete macOS-first interaction acceptance.

## Recorded candidate findings

- [macOS application/Dock icon behavior, issue #1](https://github.com/avacadorun-dev/writer-craft/issues/1): owner-reported current-alpha bug; track the fix and observed packaged-app result. Recording this item does not state it is resolved.
- Persistent visual text marks with attached notes/reminder text: future legacy-parity enhancement, explicitly outside `0.1.0-alpha.1`. See [migration brief](../migration/PERSISTENT_ANNOTATIONS.md) and [proposed OpenSpec change](../../openspec/changes/persistent-text-annotations/proposal.md).

Issue #1 and [annotation issue #2](https://github.com/avacadorun-dev/writer-craft/issues/2) were created from the owner's `avacadorun-dev` account, with `bug` and `enhancement` labels respectively. The enhancement was filed through the authenticated GitHub CLI and is not yet selected for a release.

## Joint scope decision

After the owner tests the first alpha:

1. Review owner findings and duplicate-triaged GitHub issues, including severity, reproducibility and user impact.
2. Separate current-release defects, deferred source-parity work and unresolved design hypotheses.
3. Compare the two paths against data preservation, editor/anchor dependencies and independent UX evidence.
4. Record the chosen scope, exclusions, version and acceptance tasks in the selected change. Keep unchosen candidates open without starting their implementation.
5. Implement only that agreed scope and run the existing source/release gates against actual resulting evidence.

## Planning verification

The persistent-annotation change is documentation-only at this point. On 2026-11-08, the pinned official `@fission-ai/openspec@1.16.1` CLI ran `openspec validate --all --strict --json`: **7 of 7 items passed**, comprising five canonical specs and two proposed changes, with zero validation issues. The `persistent-text-annotations` change passed. This verifies document structure and specification rules; no annotation implementation test or native acceptance is claimed.

Reproduce with `npx --yes --package @fission-ai/openspec@1.16.1 openspec validate --all --strict --json` from the repository root. Framework: [Fission-AI/OpenSpec](https://github.com/Fission-AI/OpenSpec).

Tracking: [persistent text marks and author notes — issue #2](https://github.com/avacadorun-dev/writer-craft/issues/2).
