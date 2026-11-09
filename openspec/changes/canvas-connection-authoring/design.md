# Host interaction and durable edge commands

Keep the acknowledged CanvasSession as authority. Place connection UI in a private Canvas child module. Use existing EditLabel/Delete engine commands and the existing prepare → durable draft/save → acknowledgement path; no new storage model or engine pin. Label drafts capture exact edge identity, label and session base; stale/pending operations refuse before mutation, and cancellation discards only an unsubmitted label draft. Existing failed transaction/retry preserves submitted candidates.

Host tool clicks capture the tool before toolbar processing, require press and release inside the allocated clipped board on its active UI layer, and use egui click recognition to exclude drags. Clear transient press on release, tool switch, cancellation or blocked input. Resolve overlapping cards in reverse paint order. Preserve the Select tool's existing empty-board double click. A temporary connection line is paint-only. Existing edges are reachable through a bounded, localized connection list with a label editor and delete action.

Follow native-canvas-integration authority and F-42/F-45. Engine mechanics remain in craft-canvas; app-specific selection, labels and localization stay here. No public raw user fixtures.
