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
