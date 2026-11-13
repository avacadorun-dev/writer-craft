# Author the saved imported project structure through shared commands

## Why

Displaying the imported hierarchy does not restore the original workflow of creating the allowed sections/documents or writing into a content-bearing node without saved text. Issue #10 requires usable original custom structures and stable identities; #14 separately governs future creation profiles. A flat Add document action must not silently bypass an imported project's rules.

## What changes

Expose the imported structure as a scoped snapshot. Create a root/child using its current saved kind and placement rules, and initialize first content in an existing content-bearing node under an exact metadata guard. Keep original import evidence immutable. One transaction publishes metadata, scoped aliases, a canonical rich head and its first history revision; contentless sections receive no invented text document.

## Impact

This is existing-source workflow restoration, not a new project taxonomy. Move/reorder/trash/restore, schema editing and native menus remain required subsequent work. Core checks and native round trips are separate from actual independent UI acceptance.
