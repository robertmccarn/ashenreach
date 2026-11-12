# Ashen Reach content

The `content/` tree is the authored data source loaded by `ContentManager` at startup and by the debug `reload_content` command. Content is separate from save data: `manifest.json` declares the content format only.

## Directory layout

```text
content/
  manifest.json
  schemas/       # handwritten JSON Schemas
  items/
  enemies/
  loot_tables/
  encounters/
  maps/
```

Each JSON file contains one definition. The filename is not the identity; the lowercase `id` field is the stable runtime key. IDs may contain lowercase letters, digits, `_`, and `-`, and must be unique within their content type.

## Manifest and schemas

`manifest.json` must contain exactly `{ "format_version": 1 }`. Missing, malformed, unsupported, or structurally invalid manifests stop loading. The five handwritten schemas in `schemas/` define the serialized shape and reject unknown fields, missing required fields, invalid types, and values outside their declared numeric ranges. The loader validates a document against its matching schema before deserializing the Rust DTO.

## Semantic validation

After deserialization, the loader checks finite and positive values, non-negative item weights, valid damage ranges, non-empty loot tables, positive loot weights and rolls, positive encounter counts, and all cross-references: enemies point to loot tables, loot entries point to items, and encounter spawns point to enemies. Duplicate or invalid IDs and every error retain the source file path and field context.

## Maps

Maps use rectangular rows containing only these glyphs:

| Glyph | Meaning | Connectivity |
| --- | --- | --- |
| `#` | Wall | Blocked |
| `.` | Floor | Traversable |
| `D` | Door | Traversable for validation; closed at runtime |
| `S` | Start | Traversable; exactly one required |
| `X` | Exit | Traversable; at least one required |

Every exit must be reachable from the start. Content validation and runtime `World::from_map()` share the same flood-fill helper, so a malformed map is rejected even when constructed without the content loader. Diagnostics include the map file, ID, and coordinates for unsupported tiles and unreachable exits.

## Adding content safely

1. Choose a unique valid `id`.
2. Add one JSON definition to the appropriate directory.
3. Check it against the matching schema and preserve all required fields.
4. Resolve every referenced ID in the same content set.
5. For maps, verify row widths, glyphs, one start, at least one exit, and connectivity.
6. Run `cargo fmt --all -- --check`, `cargo check`, and `cargo test` from the repository root.

At runtime, use the backtick console command `reload_content` to reload authored data. The manager builds and validates a complete replacement first; a failed reload leaves the previously loaded valid content and game state in place.
