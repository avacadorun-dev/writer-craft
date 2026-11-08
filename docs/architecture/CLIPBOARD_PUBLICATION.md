# Clipboard publication order

Status: implemented source checkpoint; hosted app tests and mounted native acceptance remain separate gates. Product issues remain open for owner acceptance.

## One document worker and native widget output

Rich Copy/Cut prepare immutable canonical content and project image capsules on one bounded serial worker. A matching provider acknowledgement is required before Cut can mutate its captured, still-current document selection. Paste prepares the original captured context once, applies host-authorized image bindings only, and completes through the original one-shot guard. No equal-plaintext cache is used as clipboard identity.

Ordinary TextEdit and Copy-code/image output continue using egui's native integration path. [egui OutputCommand](https://docs.rs/egui/0.36.2/egui/enum.OutputCommand.html) defines CopyText and CopyImage as commands executed by the integration at the end of a frame. If an older rich native write is still pending, emitting a widget command immediately could let that older write overwrite the user's newer Copy.

The host therefore retains the exact owned OutputCommand and removes it from that frame. A publication fence synchronizes only the worker's final cancellation check and native-write start. It never covers project/SQLite work, image preparation, provider I/O or a thread join. The UI uses try_lock; contention keeps the command for a later frame. The worker publishes active-write state before releasing the fence and clears it only when the actual provider call has returned. Cancellation recorded under the same fence prevents older queued/preparing jobs from starting a later write.

Only after that older write ends, and only if no newer Copy intent superseded it, does the host re-emit the exact egui command. Taking or retaining a command is not a provider acknowledgement. Framework output is reported as issued only after actual re-emission. Rich Cut continues requiring its original sealed provider ACK; this widget barrier cannot fabricate one.

## Cancellation and failure

Document/modal cancellation invalidates Cut/Paste and old rich publications. An independently captured native widget Copy has no manuscript destination, so repeated dialog frames or a project switch do not repeatedly cancel it. A newer Copy/Cut publication intent supersedes older pending Copy output in order. Text and image payloads are retained without conversion or lossy reconstruction.

Paste is refused with the existing localized pending status while either a rich write or retained widget output is unpublished. It cannot consume an older OS buffer as though the newer Copy had completed. A publication-handshake error preserves the exact widget command and offers the existing explicit Copy captured content/Dismiss captured content actions. The implementation never clears previously owned OS clipboard data to signal cancellation or failure.

## Evidence and boundary

Blocked-provider fixtures verify that old writes finish before widget output becomes available; repeated document cancellation does not discard independent Copy; newer widget/image and rich intents win in order; old-write failure does not clear existing clipboard bytes; try_lock contention never waits; and error/retry preserves the exact command. Frame fixtures verify that Paste is blocked until re-emission, exact text/image output survives, and ordinary widget Copy retains its normal path when no rich write is pending.

These protocol/frame fixtures do not constitute a mounted OS UI test. The engine's separate hosted macOS/X11 nonce probes verify native transport/format roundtrips. Background transport, complete native keymaps, OS ownership changes and full writing workflows require their own observed acceptance; no product issue is closed from this source checkpoint alone.
