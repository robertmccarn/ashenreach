//! Versioned, validated authored content. Runtime state belongs in simulation;
//! this module only owns immutable definitions loaded from `content/`.

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
        result.items = load_directory(&root, "items", "item", &mut errors);
        result.enemies = load_directory(&root, "enemies", "enemy", &mut errors);
        result.loot_tables = load_directory(&root, "loot_tables", "loot_table", &mut errors);
        result.encounters = load_directory(&root, "encounters", "encounter", &mut errors);
        result.maps = load_directory(&root, "maps", "map", &mut errors);
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

    fn validate(&self, errors: &mut Vec<String>) {
        for enemy in self.enemies.values() {
            if enemy.stats.health == 0
                || enemy.stats.move_speed <= 0.0
                || !enemy.stats.move_speed.is_finite()
            {
                errors.push(format!("enemies/{}.json: stats.health must be > 0 and stats.move_speed must be finite and > 0", enemy.id));
            }
            if !self.loot_tables.contains_key(&enemy.loot_table_id) {
                errors.push(format!(
                    "enemies/{}.json: loot_table_id `{}` does not exist in loot_tables",
                    enemy.id, enemy.loot_table_id
                ));
            }
        }
        for loot in self.loot_tables.values() {
            if loot.rolls == 0 || loot.entries.is_empty() {
                errors.push(format!(
                    "loot_tables/{}.json: rolls and entries must both be non-zero",
                    loot.id
                ));
            }
            for entry in &loot.entries {
                if entry.weight == 0 {
                    errors.push(format!(
                        "loot_tables/{}.json: item `{}` has zero weight",
                        loot.id, entry.item_id
                    ));
                }
                if !self.items.contains_key(&entry.item_id) {
                    errors.push(format!(
                        "loot_tables/{}.json: item_id `{}` does not exist in items",
                        loot.id, entry.item_id
                    ));
                }
            }
        }
        for encounter in self.encounters.values() {
            if encounter.enemies.is_empty() {
                errors.push(format!(
                    "encounters/{}.json: enemies must not be empty",
                    encounter.id
                ));
            }
            for spawn in &encounter.enemies {
                if spawn.count == 0 {
                    errors.push(format!(
                        "encounters/{}.json: enemy `{}` has count 0",
                        encounter.id, spawn.enemy_id
                    ));
                }
                if !self.enemies.contains_key(&spawn.enemy_id) {
                    errors.push(format!(
                        "encounters/{}.json: enemy_id `{}` does not exist in enemies",
                        encounter.id, spawn.enemy_id
                    ));
                }
            }
        }
        for item in self.items.values() {
            if item.name.trim().is_empty() || !item.weight.is_finite() || item.weight < 0.0 {
                errors.push(format!(
                    "items/{}.json: name is required and weight must be finite and >= 0",
                    item.id
                ));
            }
            if let Some(damage) = &item.damage {
                if damage.min > damage.max {
                    errors.push(format!(
                        "items/{}.json: damage.min must be <= damage.max",
                        item.id
                    ));
                }
            }
        }
        for map in self.maps.values() {
            validate_map(map, errors);
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

fn validate_map(map: &MapDefinition, errors: &mut Vec<String>) {
    if map.rows.is_empty() {
        errors.push(format!("maps/{}.json: rows must not be empty", map.id));
        return;
    }
    let width = map.rows[0].len();
    let mut starts = 0;
    let mut exits = 0;
    for (line, row) in map.rows.iter().enumerate() {
        if row.len() != width {
            errors.push(format!(
                "maps/{}.json: rows[{line}] has width {}, expected {width}",
                map.id,
                row.len()
            ));
        }
        for cell in row.bytes() {
            match cell {
                b'#' | b'.' | b'D' => {}
                b'S' => starts += 1,
                b'X' => exits += 1,
                _ => errors.push(format!(
                    "maps/{}.json: rows[{line}] has unsupported tile `{}`",
                    map.id, cell as char
                )),
            }
        }
    }
    if starts != 1 {
        errors.push(format!(
            "maps/{}.json: exactly one `S` start is required; found {starts}",
            map.id
        ));
    }
    if exits == 0 {
        errors.push(format!(
            "maps/{}.json: at least one `X` exit is required",
            map.id
        ));
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
}
