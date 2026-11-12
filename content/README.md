# Ashen Reach authored content

Each content root must contain `manifest.json` with a supported `format_version`. The current content contract is version `1`; this is independent of any future save-game format version. Missing, malformed, or unsupported manifests stop loading before definitions are accepted.

Each `.json` file contains one definition. File names are only for people; the lowercase `id` field is the stable runtime key. IDs may contain lowercase letters, digits, `_`, and `-`. The loader validates every definition against its matching JSON Schema in `schemas/` before deserializing it into the Rust DTO. The DTOs also deny unknown fields. Schema validation, Rust-side numeric checks, duplicate-ID checks, and cross-reference checks all run before a `ContentManager` is returned.

Maps must be rectangular and use only `#`, `.`, `D`, `S`, and `X`. Exactly one start and at least one exit are required. Every exit must be reachable from the start; doors count as traversable because the player can open them. Connectivity errors include the map ID and exit/start coordinates.

The content directories are `items/`, `enemies/`, `loot_tables/`, `encounters/`, and `maps/`. Add a definition by updating its DTO and matching schema together, then add or update a fixture test. The content tests exercise malformed JSON, manifest versions, schema violations (including missing required and unknown fields), duplicate IDs, broken references, invalid map dimensions and glyphs, missing starts/exits, and unreachable exits. A positive test also confirms that a door does not make an otherwise connected exit fail validation.

At startup, content is validated before the game begins. During a debug session, open the backtick console and run `reload_content`. A failed load returns diagnostics and does not replace the current content manager.
