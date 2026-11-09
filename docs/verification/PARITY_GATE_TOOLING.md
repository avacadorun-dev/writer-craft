# Parity and role-journey gate verification

Date:2026-11-09. Named scope: release tooling and acceptance contracts only. No native application behavior, end-to-end product capability, user study or alpha.3 release is accepted by this record.

Source/lock/contract fingerprint reviewed: `43dddbd619b3af801989e5d9450c075ef069cfb7a63de8b54492eaf439aaa7ce`.

## Historical verified checks

- `python3 scripts/release-check.py --self-test`:19 tests passed, including missing/duplicate/unknown/stale/pending coverage, unreviewed inventories, incomplete task paths, changed or unrelated report receipts, null entry/reviewer fields, source/requirement/enforcement changes and narrower-catalog scope escalation.
- `python3 scripts/package-portable.py --self-test`:5 tests passed.
- `python3 scripts/stage-release-assets.py --self-test`:5 tests passed.
- OpenSpec1.16.1 `validate --all --strict --json`:20 items passed (14 changes,6 canonical specs).
- Current source mode passes with `release_ready:false`; current alpha.3 release mode correctly refuses pending acceptance.
- `git diff --check` passes. Historical `UX_GATE_ALPHA2_ACCEPTED.json` is unchanged. No Cargo build/fetch/test or native application control occurred locally.

## Historical independent review

`parity_gate_final_review` accepted the named tooling/contract scope at the exact fingerprint above after independently rerunning all19 Python tests, source mode and refused release mode. The previous reviewer identified false-pass risks; fixes bind report candidate/IDs/paths, restrict the supported scope, require real nonempty identity/entry strings and fingerprint both validators plus authoritative contracts.

This approval does not populate `UX_GATE.json.coverage` or mark its pending inventories/capabilities/journeys accepted. Those require actual source-omission and native interaction evidence. The current catalog is an initial scaffold, not exhaustive product verification or a completion percentage. #17 records original Canvas authoring; #15 remains the single active product implementation. Candidate engine73e4f9b is not adopted or qualified by these checks.

Hosted [Verify WriterCraft run42557568070](https://github.com/avacadorun-dev/writer-craft/actions/runs/42557568070) passed for commit `f57071f6eef3ea97952c77d47aeb14b7eaca78a4`: macOS source/engineering checks, Linux shared core and Windows shared core all succeeded. The preview packaging job was skipped as configured; no new desktop build or product acceptance is claimed.


## Issue #15 admission hardening — source candidate

The earlier tooling accepted a correctly hashed report even when its observation failed or omitted an outcome. It also did not distinguish injected egui events from operating-system input, and broad B01/B26 titles allowed input-only evidence to stand in for GFM, board history and recovery. This candidate adds fail-closed receipt requirements without changing `schema_version: 1` or marking any product coverage accepted. Existing receipt files must be updated from real observations and re-reviewed; missing fields are not inferred as passing.

### Observation receipt contract

Each matched receipt observation requires `outcome: "passed"` and explicit `provenance`. Failed, blocked, pending, skipped or absent outcomes cannot qualify. A successful failure/recovery scenario means the required safe handling was observed, not that a save failure itself is a passing product result.

| Observation kind | Allowed provenance | Meaning |
|---|---|---|
| `inventory-audit` | `source-audit` | Independent source inventory and omission review. |
| `semantic` | `automated-test` | Actual named tests of the candidate. |
| `native`, `native-journey` | `native-os` | Input delivered through the real operating-system integration, manually or through OS-level automation. |
| `native`, `native-journey` | `injected-egui` | Synthetic events delivered at egui/RawInput level; this does not exercise OS IME, clipboard or native event delivery. |
| `native` only | `compiled-ui` | Observed compiled presentation without claiming operating-system input coverage. |

The existing exact candidate fingerprint, hosted build reference, platform, narrative report hash, independently named reviewer and required evidence kinds remain mandatory. A platform/build URL or a screenshot never determines input provenance automatically. Provenance describes the observed path, not whether the fixture is synthetic: a synthetic document may be exercised through real OS input.

Journey observations retain their explicit `paths` and additionally require `path_results`, with exactly one result for every contract `required_paths` ID. Each result has its own `outcome: "passed"` and `provenance`; aggregate success cannot hide a failed, skipped or unobserved cancellation/recovery/return path. Both the journey observation and each path must use interaction provenance (`native-os` or `injected-egui`); a complete set of static `compiled-ui` observations cannot qualify a journey.

Capability contracts may declare nonempty `required_operations`. Every operation requires a stable `id`, descriptive `title` and explicit boolean `native_os_required`. Both the matching `semantic` and `native` observations must then provide `operation_results` covering exactly those IDs. Every result needs its own passed outcome and compatible provenance. Semantic tests remain required but cannot replace native evidence for an OS obligation; a native result with `native_os_required: true` accepts only `native-os`.

For example, an individual operation result is:

```json
{"id": "native-ime", "outcome": "passed", "provenance": "native-os"}
```

An individual journey-path result is:

```json
{"id": "failure-recovery", "outcome": "passed", "provenance": "native-os"}
```

These are record fragments, not complete qualifying receipts. All required operation/path IDs must be present without duplicates or unknown IDs. Each result's provenance must match its enclosing observation; an `injected-egui` observation cannot relabel one result as `native-os`. Reports may contain separate semantic/native observations with different provenance. There is no automatic mixed-provenance upgrade or missing-field fallback.

### Canvas scope boundary

B01 now names card creation, inline editing, native selection, native clipboard, native IME, rendered GFM and reopen/edit separately. B26 separately names local text Undo/Redo, board Undo/Redo, complete save/reopen, pending acknowledgements, stale rejection, failure, cancellation/preservation and restart recovery. The operation descriptions and OS obligations are authoritative in `PARITY_SCOPE.json`.

Issue #15 input evidence may establish individual operations; it cannot accept all of B01/B26, complete GFM rendering, board history/recovery, or issue #17. The generic validator has no hard-coded acceptance exception for #15 or these capability IDs. Other broad capabilities remain pending independent inventory refinement; these two decompositions do not certify that every inventory is exhaustive.

### Local verification and limits

`python3 scripts/release-check.py --self-test` passes 27 Python tests after these changes. New cases reject missing/failed observation and per-path outcomes, missing/contradictory provenance, absent/partial/duplicate/unknown operation results, malformed operation contracts, static-only journey receipts and injected/render-only evidence for OS obligations. Positive cases preserve complete semantic/native coverage and explicitly classified non-OS observations. `git diff --check` passes.

This is local tooling verification of the uncommitted source candidate, not an independent acceptance of the final integrated fingerprint. No Cargo build/test, hosted qualification or native application control was performed for this patch. `UX_GATE.json`, historical alpha.2 acceptance and all product inventory/coverage verdicts remain unchanged. The validator checks receipt consistency, not whether a claimed observation actually happened; independent review of the exact build, source and native evidence remains required.


Independent review of this bounded hardening passed all27 Python self-tests, source mode (`release_ready: false`) and correctly refused alpha.3 release mode at working-tree fingerprint `2afe173603d89cac2b6abb972bb9236a2d6fe9f69551c8ccf0b78502e7f2f68f`. The reviewer accepted the tooling delta after static `compiled-ui` journey evidence was rejected. This records that reviewed source checkpoint, not approval of later application changes or the final product fingerprint.


The published gate hardening in `18eebc6b46d9f874acb4cfd95632285a3255bedc` passed the complete [Verify42564424381](https://github.com/avacadorun-dev/writer-craft/actions/runs/42564424381) hosted workflow. This qualifies the tooling checkpoint while its product gate remains pending.
