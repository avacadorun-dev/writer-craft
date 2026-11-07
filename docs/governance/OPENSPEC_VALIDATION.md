# OpenSpec validation evidence

Date: 2026-11-07. Official npm package: [@fission-ai/openspec1.16.1](https://www.npmjs.com/package/@fission-ai/openspec/v/1.16.1). Version output: `1.16.1`.

The CLI ran through `npx --yes --package @fission-ai/openspec@1.16.1` from an isolated temporary npm cache, with `OPENSPEC_TELEMETRY=0` and `OPENSPEC_NO_UPDATE_CHECK=1`; it was not installed globally. The cache's machine-specific absolute path is omitted from this public evidence. Executed CLI operation:

```sh
openspec validate --all --strict --json
```

Result: exit0; five items passed, none failed; every item returned an empty `issues` array.

| Item | Type | Result |
|---|---|---|
| appearance | spec | valid |
| assistant-review | spec | valid |
| manuscript-editing | spec | valid |
| native-workspace | spec | valid |
| native-mvp | change | valid |

This validates artifact syntax and supported contract structure. It does not prove implementation completion, native UX, migration parity or generated agent integration. `openspec init` was not run; repository instructions and config remain hand-maintained. Future contract changes rerun the pinned validation and update this evidence when results differ.
