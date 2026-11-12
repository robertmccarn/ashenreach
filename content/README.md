# Ashen Reach authored content

Each `.json` file contains one definition. File names are only for people; the lowercase `id` field is the stable runtime key. IDs may contain lowercase letters, digits, `_`, and `-`. The loader rejects duplicate IDs, unknown fields, malformed JSON, invalid numeric ranges, invalid map rows, and broken item/enemy/loot references. Errors include the file and JSON line/column where parsing fails.

Schemas in `schemas/` document the exact object shape for editors and AI authoring. `items/`, `enemies/`, `loot_tables/`, `encounters/`, and `maps/` are independently indexed for constant-time lookup. Add a new type by defining a `Deserialize` DTO in `src/content.rs`, loading its directory into `ContentManager`, adding cross-reference checks in `validate`, and committing a matching schema plus a valid sample fixture.

At startup, content is validated before the game begins. During a debug session, open the backtick console and run `reload_content`. The manager replaces its index only after the entire new content set validates; the prototype map is then restarted from its newly loaded definition.
