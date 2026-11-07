# Contributing to WriterCraft

WriterCraft is an early native Rust migration of Writers Studio. Contributions should advance the verified [feature inventory](docs/migration/FEATURE_PARITY.md), preserve user data and improve daily desktop usability. A working engineering preview is not yet full workflow parity.

## Start here

Read [AGENTS.md](AGENTS.md), the [PRD](docs/PRD.md), [architecture decisions](docs/architecture/DECISIONS.md) and [specification workflow](docs/governance/SPEC_WORKFLOW.md). Describe behavior changes through the existing OpenSpec proposal/spec/design/tasks process. Separate implementation evidence from planned capability.

Use a descriptive branch such as `feature/…` or `fix/…`. Keep a change focused and explain the user-visible result, verification and remaining limitations. Never commit workspace databases, credentials, `target/`, generated app bundles or temporary investigation outputs.

## Build and check

Use the Rust version declared in `Cargo.toml` or newer, plus the native platform build tools described in the README.

```sh
cargo run --locked -p studio-desktop
./scripts/check.sh
python3 scripts/release-check.py --self-test
python3 scripts/package-portable.py --self-test
```

The isolated editor-model hypothesis can be reproduced without adding that dependency to the application:

```sh
CARGO_TARGET_DIR=target cargo test --locked --manifest-path tests/editor-compatibility/Cargo.toml
```

Preserve immutable revisions, recoverable drafts, exact Unicode anchors and reviewed Apply/Reject/Undo boundaries. Imported rich data must not be silently flattened. Keep all eight interface catalogs complete; document text and user-provided names are never translated automatically.

## Desktop review

UI changes need actual compiled captures and interaction verification against [UX acceptance](docs/design/UX_ACCEPTANCE.md). Record owner corrections in the feedback log. Independent acceptance is scoped and tied to source/evidence; an implementer cannot approve their own change or treat passing tests as a complete usability review.

Use the selected Lucide utility family and separate monk/book product mark. Retain third-party licenses and provenance. A reusable engine with substantial independent scope should be discussed as a separate library rather than copied wholesale into this product.

Distribution builds run on GitHub Actions. Follow [release policy](docs/governance/RELEASING.md); published tags and assets are immutable, and pending UX findings cannot be bypassed to publish a candidate.
