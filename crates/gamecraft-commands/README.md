# `gamecraft-commands` (L1 Undoable Command Bus)

Deterministic, undoable command bus over the `gamecraft-core` scene graph (`execute`, `undo`, `serialize_json`).

Every scene mutation in GameCraft — whether emitted by the depot forge, a prompt compiler, or an MCP tool — flows through typed commands so edits can be recorded to `commands.jsonl`, replayed, and undone cleanly.
