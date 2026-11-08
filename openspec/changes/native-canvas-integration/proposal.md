# Native project Canvas integration

## Why

Issue #10 requires real native use of migrated boards rather than archive preservation alone. The owner accepted the alpha.2 writing checkpoint and authorized completing the original project workflows in alpha.3.

## What changes

Integrate the separately qualified CraftCanvas model and egui adapter into the existing bounded desktop workspace. Keep WriterCraft project scope, source dialect, persistence, locale catalogs, resources and navigation in the host. Prepare each native operation against one exact generic session; commit through the existing guarded SQLite board command before acknowledging the session. Preserve source extensions and provide deliberate text editing, history and recoverable failure paths.

## Scope

This is one implementation workstream under the complete project-portability contract. It does not itself complete typed cards, drawing, full media playback, original-project private acceptance or issue #10. No WebView or copied upstream application is introduced.
