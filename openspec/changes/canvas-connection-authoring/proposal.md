# Complete native connection authoring

Current disposition (2026-11-09, F-47/F-48): the two-click/menu-first interaction below is retained historical implementation, not accepted UX. Primary creation and reconnection require direct dragging. Generic implementation moves to the standalone CraftCanvas production roadmap; this app change is parked until an exact qualified engine handoff. Full B01–B26 remains required but is explicitly deferred from the newly authorized bounded alpha.3 release.

Continue issue #17, Canvas B15 and the B01/B26 input boundary. Recover the existing connection implementation at 4bc85c2 and its uncommitted input corrections. Authors can connect the intended topmost cards, see an uncommitted connector preview, select an existing connection, edit its label and delete it. Toolbar and outside-board clicks must not create cards or connections.

Use the already qualified Canvas engine and durable revision commands. Preserve IDs, original nodes, unknown edge fields and compensating history. This bounded slice does not accept the rest of B01–B26 or the full issue. No schema/dependency migration.
