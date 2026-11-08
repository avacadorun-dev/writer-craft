# Capability folder dependency qualification

Status: lock/provenance checkpoint, 2026-11-09. Folder behavior and native UI acceptance remain pending. ADR-014 selects an established relative-directory capability instead of private unsafe platform filesystem code.

The workspace declares `cap-std =4.0.3`, `default-features = false`, used only by `studio-transfer`. Existing independent CraftEditor/Canvas/media pins and all previously retained registry versions are unchanged. The resolved graph adds eleven registry packages; the sole changed retained package record is the new `studio-transfer → cap-std` edge. All eleven registry checksums were independently matched to the exact published version metadata.

The [isolated hosted metadata job](https://github.com/avacadorun-dev/writer-craft/actions/runs/42508206569) ran helper source `fc1f751faa170fdaf87d34b42f0f169bd69b9d82` on Linux with rustc1.99.1 (`58cf8f9c3`, LLVM23.1.1). It intentionally failed the final stale-committed-lock gate after successfully resolving and retaining the lock; native build was skipped. This is dependency resolution evidence, **not** compiled folder/test qualification. Adopted Cargo.lock SHA-256: `7b6badfc62c9cbff97933370e2e938eca6c10a712a845611b0b36ec3f90fa00b`. The workspace minimum is Rust1.97; no below-minimum compatibility claim follows from package declarations.

| Exact added version | Published license expression | Declared Rust minimum |
|---|---|---|
| [ambient-authority 0.0.2](https://crates.io/crates/ambient-authority/0.0.2) | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | Unspecified |
| [cap-primitives 4.0.3](https://crates.io/crates/cap-primitives/4.0.3) | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | Unspecified |
| [cap-std 4.0.3](https://crates.io/crates/cap-std/4.0.3) | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | Unspecified |
| [fs-set-times 0.20.3](https://crates.io/crates/fs-set-times/0.20.3) | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | Unspecified |
| [io-extras 0.19.0](https://crates.io/crates/io-extras/0.19.0) | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | 1.70 |
| [io-lifetimes 2.0.4](https://crates.io/crates/io-lifetimes/2.0.4) | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | 1.63 |
| [io-lifetimes 3.0.1](https://crates.io/crates/io-lifetimes/3.0.1) | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | 1.70 |
| [ipnet 2.12.2](https://crates.io/crates/ipnet/2.12.2) | `MIT OR Apache-2.0` | Unspecified |
| [maybe-owned 0.3.4](https://crates.io/crates/maybe-owned/0.3.4) | `MIT OR Apache-2.0` | Unspecified |
| [rustix-linux-procfs 0.1.1](https://crates.io/crates/rustix-linux-procfs/0.1.1) | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | 1.63 |
| [winx 0.36.4](https://crates.io/crates/winx/0.36.4) | `Apache-2.0 WITH LLVM-exception` | 1.63 |

Retain package notices verbatim in target-resolved distribution notices. `winx` uses the Apache-2.0 WITH LLVM-exception expression and must not be relabelled MIT merely because the application license is MIT. The existing notice generator traverses normal package edges and collects declared/package-root licenses; each platform package still needs its actual generated notice inventory checked. Linux-only `rustix-linux-procfs` and Windows-only `winx` remain conditional edges.

Qualify pinned-root rename/replacement, inner symlink/ancestor refusal, regular-file guards, changed reading views, exact controls/object hashes, resource bounds and cancellation. Capture the actual native folder export/restore workflow and independent UX review separately. No universal filesystem power-loss or signing claim is made.

## Opened regular-file refinement

Independent source review found a FIFO/type substitution between path metadata and a blocking open. Qualify the same-version public `cap-fs-ext =4.0.3`, default features disabled, for `OpenOptionsFollowExt::follow(No)` and `OpenOptionsSyncExt::nonblock(true)`, then check the actual opened handle's regular-file type. Do not use hidden cap-primitives methods, copied OS flag constants or private unsafe traversal. The [versioned trait implementation](https://github.com/sunfishcode/cap-std/blob/v4.0.3/cap-fs-ext/src/open_options_sync_ext.rs) forwards the established primitive options; confinement remains cap-std's responsibility.

[Hosted metadata job42510219294](https://github.com/avacadorun-dev/writer-craft/actions/runs/42510219294), helper `afa3d76a571c5d66484211b28e9d471873a09711`, added only this single registry package and the studio-transfer edge; no retained package versions changed. Its checksum was matched to the published package metadata. Declared license: `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT`; preserve notices as before. This metadata job again intentionally failed the stale-lock gate after resolution and skipped native compilation. Opened-handle/FIFO and cross-platform qualification remain separate pending checks.
Updated lock SHA-256: `ffd1942ed931f0f2f64168c305c0369edfd052fe82cecbae39a1fc9c8e59a85c`.
