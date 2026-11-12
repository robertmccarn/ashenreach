//! Versioned, validated authored content. Runtime state belongs in simulation;
//! this module only owns immutable definitions loaded from `content/`.

use crate::reachability::flood_fill;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemDefinition {
    pub id: String,
    pub name: String,
    pub kind: ItemKind,
    pub value: u32,
    pub weight: f32,
    #[serde(default)]
    pub effects: Vec<String>,
    #[serde(default)]
    pub damage: Option<DamageRange>,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Weapon,
    Armor,
    Consumable,
    Quest,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageRange {
    pub min: u32,
    pub max: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyDefinition {
    pub id: String,
    pub name: String,
    pub stats: Stats,
    pub loot_table_id: String,
    #[serde(default)]
    pub abilities: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stats {
    pub health: u32,
    pub attack: u32,
    pub defense: u32,
    pub move_speed: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LootTableDefinition {
    pub id: String,
    pub rolls: u32,
    pub entries: Vec<LootEntry>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LootEntry {
    pub item_id: String,
    pub weight: u32,
    #[serde(default)]
    pub quantity: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterDefinition {
    pub id: String,
    pub name: String,
    pub enemies: Vec<EncounterSpawn>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterSpawn {
    pub enemy_id: String,
    pub count: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapDefinition {
    pub id: String,
    pub name: String,
    pub rows: Vec<String>,
}

/// Central index. Every definition is immutable after a successful load.
/// `reload` builds a replacement index first, so failed reloads keep the last
/// known-good content available.
#[derive(Clone, Debug, Default)]
pub struct ContentManager {
    root: PathBuf,
    source_paths: HashMap<String, PathBuf>,
    pub items: HashMap<String, ItemDefinition>,
    pub enemies: HashMap<String, EnemyDefinition>,
    pub loot_tables: HashMap<String, LootTableDefinition>,
    pub encounters: HashMap<String, EncounterDefinition>,
    pub maps: HashMap<String, MapDefinition>,
}

impl ContentManager {
    pub fn load(root: impl Into<PathBuf>) -> Result<Self, Vec<String>> {
        let root = root.into();
        let manifest_path = root.join("manifest.json");
        let manifest_text = fs::read_to_string(&manifest_path).map_err(|error| {
            vec![format!(
                "{}: cannot read manifest: {error}",
                manifest_path.display()
            )]
        })?;
        let manifest: ContentManifest = serde_json::from_str(&manifest_text).map_err(|error| {
            vec![format!(
                "{}:{}:{}: invalid manifest: {error}",
                manifest_path.display(),
                error.line(),
                error.column()
            )]
        })?;
        if manifest.format_version != CONTENT_FORMAT_VERSION {
            return Err(vec![format!(
                "{}: unsupported format_version {}; supported version is {}",
                manifest_path.display(),
                manifest.format_version,
                CONTENT_FORMAT_VERSION
            )]);
        }

        let mut result = Self {
            root: root.clone(),
            ..Self::default()
        };
        let mut errors = Vec::new();
        result.items = load_directory(
            &root,
            "items",
            "item",
            &mut result.source_paths,
            &mut errors,
        );
        result.enemies = load_directory(
            &root,
            "enemies",
            "enemy",
            &mut result.source_paths,
            &mut errors,
        );
        result.loot_tables = load_directory(
            &root,
            "loot_tables",
            "loot_table",
            &mut result.source_paths,
            &mut errors,
        );
        result.encounters = load_directory(
            &root,
            "encounters",
            "encounter",
            &mut result.source_paths,
            &mut errors,
        );
        result.maps = load_directory(&root, "maps", "map", &mut result.source_paths, &mut errors);
        result.validate(&mut errors);
        if errors.is_empty() {
            Ok(result)
        } else {
            Err(errors)
        }
    }

    pub fn reload(&mut self) -> Result<(), Vec<String>> {
        *self = Self::load(self.root.clone())?;
        Ok(())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn item(&self, id: &str) -> Result<&ItemDefinition, String> {
        self.get(&self.items, "item", id)
    }
    pub fn enemy(&self, id: &str) -> Result<&EnemyDefinition, String> {
        self.get(&self.enemies, "enemy", id)
    }
    pub fn encounter(&self, id: &str) -> Result<&EncounterDefinition, String> {
        self.get(&self.encounters, "encounter", id)
    }
    pub fn map(&self, id: &str) -> Result<&MapDefinition, String> {
        self.get(&self.maps, "map", id)
    }

    fn get<'a, T>(
        &'a self,
        index: &'a HashMap<String, T>,
        kind: &str,
        id: &str,
    ) -> Result<&'a T, String> {
        index
            .get(id)
            .ok_or_else(|| format!("missing {kind} definition with id `{id}`"))
    }

    fn definition_path(&self, directory: &str, id: &str) -> String {
        let key = format!("{directory}/{id}");
        match self.source_paths.get(&key) {
            Some(path) => path.display().to_string(),
            None => self
                .root
                .join(directory)
                .join(format!("{id}.json"))
                .display()
                .to_string(),
        }
    }

    fn validate(&self, errors: &mut Vec<String>) {
        for enemy in self.enemies.values() {
            let path = self.definition_path("enemies", &enemy.id);
            if enemy.stats.health == 0 {
                errors.push(format!("{path}: stats.health must be > 0"));
            }
            if !enemy.stats.move_speed.is_finite() {
                errors.push(format!("{path}: stats.move_speed must be finite"));
            } else if enemy.stats.move_speed <= 0.0 {
                errors.push(format!("{path}: stats.move_speed must be > 0"));
            }
            if !self.loot_tables.contains_key(&enemy.loot_table_id) {
                errors.push(format!(
                    "{path}: loot_table_id `{}` does not exist in loot_tables",
                    enemy.loot_table_id
                ));
            }
        }
        for loot in self.loot_tables.values() {
            let path = self.definition_path("loot_tables", &loot.id);
            if loot.rolls == 0 || loot.entries.is_empty() {
                if loot.rolls == 0 {
                    errors.push(format!("{path}: rolls must be > 0"));
                }
                if loot.entries.is_empty() {
                    errors.push(format!("{path}: entries must not be empty"));
                }
            }
            for (index, entry) in loot.entries.iter().enumerate() {
                if entry.weight == 0 {
                    errors.push(format!("{path}: entries[{index}].weight must be > 0"));
                }
                if !self.items.contains_key(&entry.item_id) {
                    errors.push(format!(
                        "{path}: entries[{index}].item_id `{}` does not exist in items",
                        entry.item_id,
                    ));
                }
            }
        }
        for encounter in self.encounters.values() {
            let path = self.definition_path("encounters", &encounter.id);
            if encounter.enemies.is_empty() {
                errors.push(format!("{path}: enemies must not be empty"));
            }
            for (index, spawn) in encounter.enemies.iter().enumerate() {
                if spawn.count == 0 {
                    errors.push(format!("{path}: enemies[{index}].count must be > 0"));
                }
                if !self.enemies.contains_key(&spawn.enemy_id) {
                    errors.push(format!(
                        "{path}: enemies[{index}].enemy_id `{}` does not exist in enemies",
                        spawn.enemy_id,
                    ));
                }
            }
        }
        for item in self.items.values() {
            let path = self.definition_path("items", &item.id);
            if item.name.trim().is_empty() {
                errors.push(format!("{path}: name must not be empty"));
            }
            if !item.weight.is_finite() {
                errors.push(format!("{path}: weight must be finite"));
            } else if item.weight < 0.0 {
                errors.push(format!("{path}: weight must be >= 0"));
            }
            if let Some(damage) = &item.damage {
                if damage.min > damage.max {
                    errors.push(format!("{path}: damage.min must be <= damage.max"));
                }
            }
        }
        for map in self.maps.values() {
            let path = self.definition_path("maps", &map.id);
            validate_map(map, &path, errors);
        }
    }
}

const CONTENT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContentManifest {
    format_version: u32,
}

fn load_directory<T: for<'a> Deserialize<'a>>(
    root: &Path,
    directory_name: &str,
    schema_name: &str,
    source_paths: &mut HashMap<String, PathBuf>,
    errors: &mut Vec<String>,
) -> HashMap<String, T>
where
    T: HasId,
{
    let schema_path = root
        .join("schemas")
        .join(format!("{schema_name}.schema.json"));
    let schema_text = match fs::read_to_string(&schema_path) {
        Ok(text) => text,
        Err(error) => {
            errors.push(format!(
                "{}: cannot read schema: {error}",
                schema_path.display()
            ));
            return HashMap::new();
        }
    };
    let schema: serde_json::Value = match serde_json::from_str(&schema_text) {
        Ok(schema) => schema,
        Err(error) => {
            errors.push(format!(
                "{}:{}:{}: invalid schema JSON: {error}",
                schema_path.display(),
                error.line(),
                error.column()
            ));
            return HashMap::new();
        }
    };
    let validator = match jsonschema::validator_for(&schema) {
        Ok(validator) => validator,
        Err(error) => {
            errors.push(format!(
                "{}: invalid JSON Schema: {error}",
                schema_path.display()
            ));
            return HashMap::new();
        }
    };

    let directory = root.join(directory_name);
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(format!(
                "{}: cannot scan directory: {error}",
                directory.display()
            ));
            return HashMap::new();
        }
    };
    let mut values = HashMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                errors.push(format!("{}: cannot read file: {error}", path.display()));
                continue;
            }
        };
        let document: serde_json::Value = match serde_json::from_str(&text) {
            Ok(document) => document,
            Err(error) => {
                errors.push(format!(
                    "{}:{}:{}: invalid {directory_name} JSON: {error}",
                    path.display(),
                    error.line(),
                    error.column()
                ));
                continue;
            }
        };
        for error in validator.iter_errors(&document) {
            errors.push(format!(
                "{}: schema validation failed at {}: {error}",
                path.display(),
                error.instance_path
            ));
        }
        match serde_json::from_value::<T>(document) {
            Ok(value) => {
                let id = value.id().to_owned();
                source_paths
                    .entry(format!("{directory_name}/{id}"))
                    .or_insert_with(|| path.clone());
                if !valid_id(&id) {
                    errors.push(format!(
                        "{}: id `{id}` must use lowercase letters, digits, `_`, or `-`",
                        path.display()
                    ));
                } else if values.insert(id.clone(), value).is_some() {
                    errors.push(format!(
                        "{}: duplicate {directory_name} id `{id}`",
                        path.display()
                    ));
                }
            }
            Err(error) => errors.push(format!(
                "{}: invalid {directory_name} definition: {error}",
                path.display(),
            )),
        }
    }
    values
}

trait HasId {
    fn id(&self) -> &str;
}
macro_rules! has_id { ($($type:ty),+) => { $(impl HasId for $type { fn id(&self) -> &str { &self.id } })+ }; }
has_id!(
    ItemDefinition,
    EnemyDefinition,
    LootTableDefinition,
    EncounterDefinition,
    MapDefinition
);

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' || c == b'-')
}

fn validate_map(map: &MapDefinition, path: &str, errors: &mut Vec<String>) {
    if map.rows.is_empty() {
        errors.push(format!(
            "{path}: map `{}` rows must contain at least one row",
            map.id
        ));
        return;
    }

    let grid: Vec<Vec<char>> = map.rows.iter().map(|row| row.chars().collect()).collect();
    let width = grid[0].len();
    let mut rectangular = true;
    let mut supported_glyphs = true;
    let mut starts = Vec::new();
    let mut exits = Vec::new();

    for (y, row) in grid.iter().enumerate() {
        if row.is_empty() {
            errors.push(format!(
                "{path}: map `{}` rows[{y}] must not be empty",
                map.id
            ));
        }
        if row.len() != width {
            rectangular = false;
            errors.push(format!(
                "{path}: map `{}` rows[{y}] has width {}, expected {width}",
                map.id,
                row.len()
            ));
        }
        for (x, &cell) in row.iter().enumerate() {
            match cell {
                '#' | '.' | 'D' => {}
                'S' => starts.push((x, y)),
                'X' => exits.push((x, y)),
                _ => {
                    supported_glyphs = false;
                    errors.push(format!(
                        "{path}: map `{}` has unsupported tile `{cell}` at ({x}, {y})",
                        map.id
                    ));
                }
            }
        }
    }

    if starts.len() != 1 {
        errors.push(format!(
            "{path}: map `{}` requires exactly one `S` start; found {} at {starts:?}",
            map.id,
            starts.len()
        ));
    }
    if exits.is_empty() {
        errors.push(format!(
            "{path}: map `{}` requires at least one `X` exit",
            map.id
        ));
    }

    if !rectangular || !supported_glyphs || starts.len() != 1 || exits.is_empty() || width == 0 {
        return;
    }

    let start = starts[0];
    let Some(reachable) = flood_fill(width, grid.len(), start, |x, y| grid[y][x] != '#') else {
        errors.push(format!(
            "{path}: map `{}` dimensions are too large to validate connectivity",
            map.id
        ));
        return;
    };
    for (x, y) in exits {
        if !reachable[y * width + x] {
            errors.push(format!(
                "{path}: map `{}` exit at ({x}, {y}) is unreachable from start at ({}, {})",
                map.id, start.0, start.1
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ContentManager;
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "ashen-reach-a1-content-test-{}-{id}",
                std::process::id()
            ));
            for directory in [
                "schemas",
                "items",
                "enemies",
                "loot_tables",
                "encounters",
                "maps",
            ] {
                fs::create_dir_all(root.join(directory)).unwrap();
            }
            let source_schemas = Path::new(env!("CARGO_MANIFEST_DIR")).join("content/schemas");
            for schema in ["item", "enemy", "loot_table", "encounter", "map"] {
                fs::copy(
                    source_schemas.join(format!("{schema}.schema.json")),
                    root.join("schemas").join(format!("{schema}.schema.json")),
                )
                .unwrap();
            }
            let fixture = Self(root);
            fixture.write("manifest.json", r#"{"format_version":1}"#);
            fixture.write(
                "items/item.json",
                r#"{"id":"test_item","name":"Test Item","kind":"weapon","value":1,"weight":1.0}"#,
            );
            fixture.write("enemies/enemy.json", r#"{"id":"test_enemy","name":"Test Enemy","stats":{"health":5,"attack":1,"defense":0,"move_speed":1.0},"loot_table_id":"test_loot"}"#);
            fixture.write("loot_tables/loot.json", r#"{"id":"test_loot","rolls":1,"entries":[{"item_id":"test_item","weight":1,"quantity":1}]}"#);
            fixture.write("encounters/encounter.json", r#"{"id":"test_encounter","name":"Test Encounter","enemies":[{"enemy_id":"test_enemy","count":1}]}"#);
            fixture.write(
                "maps/map.json",
                r######"{"id":"test_map","name":"Test Map","rows":["#####","#S.X#","#####"]}"######,
            );
            fixture
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.0.join(relative);
            fs::write(path, contents).unwrap();
        }

        fn errors(&self) -> Vec<String> {
            ContentManager::load(&self.0).expect_err("invalid fixture should be rejected")
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn shipped_content_passes_all_five_schemas_and_dto_deserialization() {
        let content = ContentManager::load("content").expect("shipped content should validate");
        assert!(!content.items.is_empty());
        assert!(!content.enemies.is_empty());
        assert!(!content.loot_tables.is_empty());
        assert!(!content.encounters.is_empty());
        assert!(!content.maps.is_empty());
    }

    #[test]
    fn manifest_version_one_is_accepted() {
        let fixture = Fixture::new();
        assert!(ContentManager::load(&fixture.0).is_ok());
    }

    #[test]
    fn missing_manifest_is_rejected_with_path() {
        let fixture = Fixture::new();
        fs::remove_file(fixture.0.join("manifest.json")).unwrap();
        assert!(
            fixture
                .errors()
                .iter()
                .any(|error| error.contains("manifest.json"))
        );
    }

    #[test]
    fn unsupported_manifest_version_is_rejected() {
        let fixture = Fixture::new();
        fixture.write("manifest.json", r#"{"format_version":2}"#);
        assert!(
            fixture
                .errors()
                .iter()
                .any(|error| error.contains("unsupported format_version"))
        );
    }

    #[test]
    fn malformed_manifest_is_rejected_with_path() {
        let fixture = Fixture::new();
        fixture.write("manifest.json", "{");
        assert!(
            fixture
                .errors()
                .iter()
                .any(|error| error.contains("manifest.json"))
        );
    }

    #[test]
    fn malformed_content_document_reports_file_line_and_column() {
        let fixture = Fixture::new();
        fixture.write("items/item.json", "{");

        let errors = fixture.errors().join("\n").replace('\\', "/");
        assert!(errors.contains("items/item.json:1:1"), "{errors}");
        assert!(errors.contains("invalid items JSON"), "{errors}");
    }

    #[test]
    fn missing_manifest_version_is_rejected() {
        let fixture = Fixture::new();
        fixture.write("manifest.json", "{}");
        assert!(
            fixture
                .errors()
                .iter()
                .any(|error| error.contains("format_version"))
        );
    }

    #[test]
    fn failed_reload_preserves_the_last_valid_content() {
        let fixture = Fixture::new();
        let mut content = ContentManager::load(&fixture.0).expect("fixture should load");
        assert!(content.items.contains_key("test_item"));
        assert!(content.maps.contains_key("test_map"));

        fixture.write("items/item.json", "{");
        let errors = content.reload().expect_err("malformed reload must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.replace('\\', "/").contains("items/item.json"))
        );
        assert!(content.items.contains_key("test_item"));
        assert!(content.maps.contains_key("test_map"));
    }

    #[test]
    fn schema_rejects_unknown_fields_with_file_context() {
        let fixture = Fixture::new();
        fixture.write("items/item.json", r#"{"id":"test_item","name":"Test Item","kind":"weapon","value":1,"weight":1.0,"extra":true}"#);
        let errors = fixture.errors().join("\n");
        assert!(errors.contains("item.json"));
        assert!(errors.contains("extra") || errors.contains("schema validation"));
    }

    #[test]
    fn schema_rejects_missing_required_fields() {
        let fixture = Fixture::new();
        fixture.write(
            "items/item.json",
            r#"{"id":"test_item","name":"Test Item","kind":"weapon","weight":1.0}"#,
        );
        assert!(
            fixture
                .errors()
                .iter()
                .any(|error| error.contains("item.json"))
        );
    }

    #[test]
    fn schema_rejects_invalid_property_types() {
        let fixture = Fixture::new();
        fixture.write(
            "items/item.json",
            r#"{"id":"test_item","name":"Test Item","kind":"weapon","value":"one","weight":1.0}"#,
        );
        assert!(
            fixture
                .errors()
                .iter()
                .any(|error| error.contains("item.json"))
        );
    }

    #[test]
    fn schema_rejects_invalid_numeric_constraints() {
        let fixture = Fixture::new();
        fixture.write(
            "items/item.json",
            r#"{"id":"test_item","name":"Test Item","kind":"weapon","value":1,"weight":-1.0}"#,
        );
        assert!(
            fixture
                .errors()
                .iter()
                .any(|error| error.contains("item.json"))
        );
    }

    fn diagnostics(fixture: &Fixture) -> String {
        fixture.errors().join("\n").replace('\\', "/")
    }

    #[test]
    fn invalid_ids_report_the_source_file_and_id_field() {
        let fixture = Fixture::new();
        fixture.write(
            "items/item.json",
            r#"{"id":"Bad ID","name":"Bad Item","kind":"weapon","value":1,"weight":1.0}"#,
        );

        let errors = diagnostics(&fixture);
        assert!(errors.contains("items/item.json"), "{errors}");
        assert!(errors.contains("id `Bad ID`"), "{errors}");
    }

    #[test]
    fn duplicate_ids_report_the_duplicate_source_file() {
        let fixture = Fixture::new();
        fixture.write(
            "items/duplicate.json",
            r#"{"id":"test_item","name":"Duplicate","kind":"weapon","value":1,"weight":1.0}"#,
        );

        let errors = diagnostics(&fixture);
        assert!(
            errors.contains("items/item.json: duplicate items id `test_item`")
                || errors.contains("items/duplicate.json: duplicate items id `test_item`"),
            "{errors}"
        );
    }

    #[test]
    fn missing_enemy_loot_table_reference_reports_its_field() {
        let fixture = Fixture::new();
        fixture.write(
            "enemies/enemy.json",
            r#"{"id":"test_enemy","name":"Test Enemy","stats":{"health":5,"attack":1,"defense":0,"move_speed":1.0},"loot_table_id":"missing_loot"}"#,
        );

        let errors = diagnostics(&fixture);
        assert!(errors.contains("enemies/enemy.json"), "{errors}");
        assert!(errors.contains("loot_table_id `missing_loot`"), "{errors}");
    }

    #[test]
    fn missing_loot_item_reference_reports_its_field() {
        let fixture = Fixture::new();
        fixture.write(
            "loot_tables/loot.json",
            r#"{"id":"test_loot","rolls":1,"entries":[{"item_id":"missing_item","weight":1,"quantity":1}]}"#,
        );

        let errors = diagnostics(&fixture);
        assert!(errors.contains("loot_tables/loot.json"), "{errors}");
        assert!(
            errors.contains("entries[0].item_id `missing_item`"),
            "{errors}"
        );
    }

    #[test]
    fn missing_encounter_enemy_reference_reports_its_field() {
        let fixture = Fixture::new();
        fixture.write(
            "encounters/encounter.json",
            r#"{"id":"test_encounter","name":"Test Encounter","enemies":[{"enemy_id":"missing_enemy","count":1}]}"#,
        );

        let errors = diagnostics(&fixture);
        assert!(errors.contains("encounters/encounter.json"), "{errors}");
        assert!(
            errors.contains("enemies[0].enemy_id `missing_enemy`"),
            "{errors}"
        );
    }

    #[test]
    fn item_weight_and_damage_range_constraints_report_fields() {
        let fixture = Fixture::new();
        fixture.write(
            "items/item.json",
            r#"{"id":"test_item","name":"Test Item","kind":"weapon","value":1,"weight":-1.0,"damage":{"min":8,"max":2}}"#,
        );

        let errors = diagnostics(&fixture);
        assert!(errors.contains("items/item.json"), "{errors}");
        assert!(errors.contains("weight must be >= 0"), "{errors}");
        assert!(
            errors.contains("damage.min must be <= damage.max"),
            "{errors}"
        );
    }

    #[test]
    fn non_finite_item_weight_is_rejected() {
        let fixture = Fixture::new();
        fixture.write(
            "items/item.json",
            r#"{"id":"test_item","name":"Test Item","kind":"weapon","value":1,"weight":1e100}"#,
        );

        let errors = diagnostics(&fixture);
        assert!(errors.contains("items/item.json"), "{errors}");
        assert!(errors.contains("weight must be finite"), "{errors}");
    }

    #[test]
    fn enemy_health_and_movement_speed_constraints_report_fields() {
        let fixture = Fixture::new();
        fixture.write(
            "enemies/enemy.json",
            r#"{"id":"test_enemy","name":"Test Enemy","stats":{"health":0,"attack":1,"defense":0,"move_speed":0},"loot_table_id":"test_loot"}"#,
        );

        let errors = diagnostics(&fixture);
        assert!(errors.contains("enemies/enemy.json"), "{errors}");
        assert!(errors.contains("stats.health must be > 0"), "{errors}");
        assert!(errors.contains("stats.move_speed must be > 0"), "{errors}");
    }

    #[test]
    fn non_finite_enemy_movement_speed_is_rejected() {
        let fixture = Fixture::new();
        fixture.write(
            "enemies/enemy.json",
            r#"{"id":"test_enemy","name":"Test Enemy","stats":{"health":5,"attack":1,"defense":0,"move_speed":1e100},"loot_table_id":"test_loot"}"#,
        );

        let errors = diagnostics(&fixture);
        assert!(errors.contains("enemies/enemy.json"), "{errors}");
        assert!(
            errors.contains("stats.move_speed must be finite"),
            "{errors}"
        );
    }

    #[test]
    fn loot_rolls_weights_and_encounter_counts_must_be_positive() {
        let fixture = Fixture::new();
        fixture.write(
            "loot_tables/loot.json",
            r#"{"id":"test_loot","rolls":0,"entries":[{"item_id":"test_item","weight":0,"quantity":1}]}"#,
        );
        fixture.write(
            "encounters/encounter.json",
            r#"{"id":"test_encounter","name":"Test Encounter","enemies":[{"enemy_id":"test_enemy","count":0}]}"#,
        );

        let errors = diagnostics(&fixture);
        assert!(
            errors.contains("loot_tables/loot.json: rolls must be > 0"),
            "{errors}"
        );
        assert!(errors.contains("entries[0].weight must be > 0"), "{errors}");
        assert!(errors.contains("encounters/encounter.json"), "{errors}");
        assert!(errors.contains("enemies[0].count must be > 0"), "{errors}");
    }

    #[test]
    fn loot_tables_require_at_least_one_entry() {
        let fixture = Fixture::new();
        fixture.write(
            "loot_tables/loot.json",
            r#"{"id":"test_loot","rolls":1,"entries":[]}"#,
        );

        let errors = diagnostics(&fixture);
        assert!(errors.contains("loot_tables/loot.json"), "{errors}");
        assert!(errors.contains("entries must not be empty"), "{errors}");
    }

    #[test]
    fn empty_map_rows_report_map_file_and_row() {
        let fixture = Fixture::new();
        fixture.write(
            "maps/map.json",
            r#"{"id":"test_map","name":"Test Map","rows":["S.X",""]}"#,
        );

        let errors = diagnostics(&fixture);
        assert!(errors.contains("maps/map.json"), "{errors}");
        assert!(errors.contains("rows[1] must not be empty"), "{errors}");
    }

    #[test]
    fn ragged_map_rows_report_the_map_and_row_width() {
        let fixture = Fixture::new();
        fixture.write(
            "maps/map.json",
            r###"{"id":"test_map","name":"Test Map","rows":["S.X","##"]}"###,
        );

        let errors = diagnostics(&fixture);
        assert!(errors.contains("maps/map.json"), "{errors}");
        assert!(
            errors.contains("rows[1] has width 2, expected 3"),
            "{errors}"
        );
    }

    #[test]
    fn unsupported_map_glyph_reports_its_coordinate() {
        let fixture = Fixture::new();
        fixture.write(
            "maps/map.json",
            r#"{"id":"test_map","name":"Test Map","rows":["S?X"]}"#,
        );

        let errors = diagnostics(&fixture);
        assert!(errors.contains("maps/map.json"), "{errors}");
        assert!(
            errors.contains("unsupported tile `?` at (1, 0)"),
            "{errors}"
        );
    }

    #[test]
    fn map_requires_one_start_and_at_least_one_exit() {
        let fixture = Fixture::new();
        fixture.write(
            "maps/map.json",
            r#"{"id":"test_map","name":"Test Map","rows":["..X"]}"#,
        );
        let errors = diagnostics(&fixture);
        assert!(
            errors.contains("requires exactly one `S` start; found 0"),
            "{errors}"
        );

        fixture.write(
            "maps/map.json",
            r#"{"id":"test_map","name":"Test Map","rows":["SSX"]}"#,
        );
        let errors = diagnostics(&fixture);
        assert!(
            errors.contains("requires exactly one `S` start; found 2"),
            "{errors}"
        );

        fixture.write(
            "maps/map.json",
            r#"{"id":"test_map","name":"Test Map","rows":["S.."]}"#,
        );
        let errors = diagnostics(&fixture);
        assert!(
            errors.contains("requires at least one `X` exit"),
            "{errors}"
        );
    }

    #[test]
    fn every_map_exit_must_be_reachable_from_the_start() {
        let fixture = Fixture::new();
        fixture.write(
            "maps/map.json",
            r#"{"id":"test_map","name":"Test Map","rows":["S#X"]}"#,
        );

        let errors = diagnostics(&fixture);
        assert!(errors.contains("maps/map.json"), "{errors}");
        assert!(
            errors.contains("map `test_map` exit at (2, 0) is unreachable from start at (0, 0)"),
            "{errors}"
        );

        fixture.write(
            "maps/map.json",
            r#"{"id":"test_map","name":"Test Map","rows":["S.X#X"]}"#,
        );
        let errors = diagnostics(&fixture);
        assert!(
            errors.contains("map `test_map` exit at (4, 0) is unreachable"),
            "{errors}"
        );
    }

    #[test]
    fn multiple_reachable_exits_are_accepted() {
        let fixture = Fixture::new();
        fixture.write(
            "maps/map.json",
            r#"{"id":"test_map","name":"Test Map","rows":["SXX"]}"#,
        );

        assert!(ContentManager::load(&fixture.0).is_ok());
    }

    #[test]
    fn all_exits_connected_through_a_door_are_accepted() {
        let fixture = Fixture::new();
        fixture.write(
            "maps/map.json",
            r#########"{"id":"test_map","name":"Test Map","rows":["########","#S..#..#","#...D.X#","#...#..#","########"]}"#########,
        );

        let content = ContentManager::load(&fixture.0).expect("door is traversable for validation");
        let world = crate::world::World::from_map(content.maps.get("test_map").unwrap()).unwrap();
        assert!(world.reachable_exit_from_start());
    }

    #[test]
    fn boundary_exits_and_small_maps_are_supported() {
        let fixture = Fixture::new();
        fixture.write(
            "maps/map.json",
            r#"{"id":"test_map","name":"Test Map","rows":["SX"]}"#,
        );

        assert!(ContentManager::load(&fixture.0).is_ok());
    }

    #[test]
    fn empty_and_blocked_maps_have_actionable_errors() {
        let fixture = Fixture::new();
        fixture.write(
            "maps/map.json",
            r#"{"id":"test_map","name":"Test Map","rows":[]}"#,
        );
        let errors = diagnostics(&fixture);
        assert!(errors.contains("maps/map.json"), "{errors}");
        assert!(
            errors.contains("rows must contain at least one row"),
            "{errors}"
        );

        fixture.write(
            "maps/map.json",
            r####"{"id":"test_map","name":"Test Map","rows":["###"]}"####,
        );
        let errors = diagnostics(&fixture);
        assert!(
            errors.contains("requires exactly one `S` start"),
            "{errors}"
        );
        assert!(
            errors.contains("requires at least one `X` exit"),
            "{errors}"
        );
    }
}
