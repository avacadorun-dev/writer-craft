# Editor compatibility qualification

A test-only, independently locked workspace for the document-model hypothesis. It is not linked into WriterCraft or published as a library.

```sh
CARGO_TARGET_DIR=target cargo test --manifest-path tests/editor-compatibility/Cargo.toml --locked
```

The synthetic fixtures exercise block identity/source attributes, UTF-16/scalar conversions, edit maps and known parser limitations. They contain no author projects. Passing these tests does not establish a complete native rich editor or lossless migration of every legacy schema.

See `docs/research/EDITOR_SPIKE.md` for the result and remaining acceptance work.
