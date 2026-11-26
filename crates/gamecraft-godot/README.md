# `gamecraft-godot` (L1 Godot 4 `.tscn`/`.tres` Parser & Emitter)

Deterministic `nom` parser, UID hasher, and scene emitter for Godot 4 `.tscn` and `.tres` text resources.

Ensures that compiling the same `spec.json` produces byte-identical Godot 4 scenes with valid node hierarchies, resource IDs, and script attachments — eliminating LLM GDScript syntax hallucinations.
