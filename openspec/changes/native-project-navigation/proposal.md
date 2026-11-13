# Restore source-backed project hierarchy in native navigation

## Why

The current native navigator flattens imported project nodes into a document list, hiding contentless sections and their saved parent/order relationships. Issue #10 requires usable original project structure; issue #14 separately researches future creation profiles. Preserving metadata in an export alone does not make imported organization usable.

## What changes

Render the existing saved hierarchy, contentless sections and deterministic sibling order in a bounded native component. Search keeps matching ancestry visible; collapse state is presentation only; opening text uses the exact project-scoped native document handle. Keep unmapped new native documents reachable without exposing trashed imported nodes as fallback documents. Add focused source-model and native event regressions.

## Impact

This does not select a new project-type taxonomy or claim complete hierarchy authoring. Source-compatible create/move/reorder/trash/restore and dynamic-schema controls remain required follow-up work in #10. No source data, imported metadata, manuscript revision or relationship may be mutated by navigation.
