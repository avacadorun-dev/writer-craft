# Complete native connection authoring

Continue issue #17, Canvas B15 and the B01/B26 input boundary. Recover the existing connection implementation at 4bc85c2 and its uncommitted input corrections. Authors can connect the intended topmost cards, see an uncommitted connector preview, select an existing connection, edit its label and delete it. Toolbar and outside-board clicks must not create cards or connections.

Use the already qualified Canvas engine and durable revision commands. Preserve IDs, original nodes, unknown edge fields and compensating history. This bounded slice does not accept the rest of B01–B26 or the full issue. No schema/dependency migration.
