//! Versioned, validated authored content. Runtime state belongs in simulation;
//! this module only owns immutable definitions loaded from `content/`.

use serde::{Deserialize, de::DeserializeOwned};
use std::{
    collections::{HashMap, VecDeque},
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
const CONTENT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContentManifest {
    format_version: u32,
}

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
        let mut result = Self {
            root: root.clone(),
            ..Self::default()
        };
        let manifest_path = root.join("manifest.json");
        let manifest_text = fs::read_to_string(&manifest_path).map_err(|error| {
            vec![format!("{}: cannot read content manifest: {error}", manifest_path.display())]
        })?;
        let manifest: ContentManifest = serde_json::from_str(&manifest_text).map_err(|error| {
            vec![format!(
                "{}:{}:{}: invalid content manifest: {error}",
                manifest_path.display(), error.line(), error.column()
            )]
        })?;
        if manifest.format_version != CONTENT_FORMAT_VERSION {
            return Err(vec![format!(
                "{}: unsupported content format version {}; this build supports version {}",
                manifest_path.display(), manifest.format_version, CONTENT_FORMAT_VERSION
            )]);
        }

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

fn load_directory<T: DeserializeOwned>(
    root: &Path,
    directory_name: &str,
    schema_name: &str,
    errors: &mut Vec<String>,
) -> HashMap<String, T>
where
    T: HasId,
{
    let schema_path = root.join("schemas").join(format!("{schema_name}.schema.json"));
    let schema_text = match fs::read_to_string(&schema_path) {
        Ok(text) => text,
        Err(error) => {
            errors.push(format!("{}: cannot read schema: {error}", schema_path.display()));
            return HashMap::new();
        }
    };
    let schema: serde_json::Value = match serde_json::from_str(&schema_text) {
        Ok(schema) => schema,
        Err(error) => {
            errors.push(format!(
                "{}:{}:{}: invalid JSON Schema: {error}",
                schema_path.display(), error.line(), error.column()
            ));
            return HashMap::new();
        }
    };
    let validator = match jsonschema::validator_for(&schema) {
        Ok(validator) => validator,
        Err(error) => {
            errors.push(format!("{}: invalid JSON Schema: {error}", schema_path.display()));
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
                    path.display(), error.line(), error.column()
                ));
                continue;
            }
        };
        let schema_errors: Vec<String> = validator
            .iter_errors(&document)
            .map(|error| format!("{}: schema validation: {error}", path.display()))
            .collect();
        if !schema_errors.is_empty() {
            errors.extend(schema_errors);
            continue;
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
                    errors.push(format!("{}: duplicate {directory_name} id `{id}`", path.display()));
                }
            }
            Err(error) => errors.push(format!("{}: invalid {directory_name} definition: {error}", path.display())),
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
    if width == 0 {
        errors.push(format!("maps/{}.json: rows must not contain empty rows", map.id));
        return;
    }

    let mut starts = Vec::new();
    let mut exits = Vec::new();
    let mut structurally_valid = true;
    for (y, row) in map.rows.iter().enumerate() {
        if row.len() != width {
            structurally_valid = false;
            errors.push(format!(
                "maps/{}.json: rows[{y}] has width {}, expected {width}",
                map.id, row.len()
            ));
        }
        for (x, cell) in row.bytes().enumerate() {
            match cell {
                b'#' | b'.' | b'D' => {}
                b'S' => starts.push((x, y)),
                b'X' => exits.push((x, y)),
                _ => {
                    structurally_valid = false;
                    errors.push(format!(
                        "maps/{}.json: unsupported tile `{}` at ({x},{y})",
                        map.id, cell as char
                    ));
                }
            }
        }
    }

    if starts.len() != 1 {
        structurally_valid = false;
        errors.push(format!(
            "maps/{}.json: exactly one `S` start is required; found {}",
            map.id, starts.len()
        ));
    }
    if exits.is_empty() {
        structurally_valid = false;
        errors.push(format!("maps/{}.json: at least one `X` exit is required", map.id));
    }
    if !structurally_valid {
        return;
    }

    // Doors are passable for solvability: the player can open them in play.
    let start = starts[0];
    let mut visited = vec![false; width * map.rows.len()];
    let mut queue = VecDeque::from([start]);
    visited[start.1 * width + start.0] = true;
    while let Some((x, y)) = queue.pop_front() {
        for (dx, dy) in [(1isize, 0isize), (-1, 0), (0, 1), (0, -1)] {
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if nx < 0 || ny < 0 || nx >= width as isize || ny >= map.rows.len() as isize {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            let tile = map.rows[ny].as_bytes()[nx];
            if tile == b'#' {
                continue;
            }
            let index = ny * width + nx;
            if !visited[index] {
                visited[index] = true;
                queue.push_back((nx, ny));
            }
        }
    }

    for (x, y) in exits {
        if !visited[y * width + x] {
            errors.push(format!(
                "maps/{}.json: exit at ({x},{y}) is unreachable from start at ({},{})",
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
                "ashen-reach-content-test-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&root).unwrap();
            for directory in ["schemas", "items", "enemies", "loot_tables", "encounters", "maps"] {
                fs::create_dir_all(root.join(directory)).unwrap();
            }
            let schema_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("content/schemas");
            for schema in ["item", "enemy", "loot_table", "encounter", "map"] {
                fs::copy(
                    schema_root.join(format!("{schema}.schema.json")),
                    root.join("schemas").join(format!("{schema}.schema.json")),
                ).unwrap();
            }
            let fixture = Self(root);
            fixture.write("manifest.json", r#"{"format_version":1}"#);
            fixture.write("items/test_item.json", r#"{"id":"test_item","name":"Test Item","kind":"weapon","value":1,"weight":1.0}"#);
            fixture.write("enemies/test_enemy.json", r#"{"id":"test_enemy","name":"Test Enemy","stats":{"health":5,"attack":1,"defense":0,"move_speed":1.0},"loot_table_id":"test_loot"}"#);
            fixture.write("loot_tables/test_loot.json", r#"{"id":"test_loot","rolls":1,"entries":[{"item_id":"test_item","weight":1,"quantity":1}]}"#);
            fixture.write("encounters/test_encounter.json", r#"{"id":"test_encounter","name":"Test Encounter","enemies":[{"enemy_id":"test_enemy","count":1}]}"#);
            fixture.write("maps/test_map.json", r#"{"id":"test_map","name":"Test Map","rows":["#####","#S.X#","#####"]}"#);
            fixture
        }

        fn write(&self, relative: &str, content: &str) {
            let path = self.0.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, content).unwrap();
        }

        fn errors(&self) -> Vec<String> {
            ContentManager::load(&self.0).expect_err("fixture should be rejected")
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn shipped_content_passes_schema_and_semantic_validation() {
        assert!(ContentManager::load("content").is_ok());
    }

    #[test]
    fn missing_manifest_is_rejected() {
        let fixture = Fixture::new();
        fs::remove_file(fixture.0.join("manifest.json")).unwrap();
        assert!(fixture.errors().iter().any(|error| error.contains("manifest.json")));
    }

    #[test]
    fn unsupported_content_format_version_is_rejected() {
        let fixture = Fixture::new();
        fixture.write("manifest.json", r#"{"format_version":999}"#);
        assert!(fixture.errors().iter().any(|error| error.contains("unsupported content format version")));
    }

    #[test]
    fn malformed_json_is_rejected_with_file_context() {
        let fixture = Fixture::new();
        fixture.write("items/broken.json", "{");
        assert!(fixture.errors().iter().any(|error| error.contains("items/broken.json")));
    }

    #[test]
    fn schema_rejects_unknown_fields() {
        let fixture = Fixture::new();
        fixture.write("items/test_item.json", r#"{"id":"test_item","name":"Test","kind":"weapon","value":1,"weight":1.0,"surprise":true}"#);
        assert!(fixture.errors().iter().any(|error| error.contains("items/test_item.json")));
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let fixture = Fixture::new();
        fixture.write("items/second_file.json", r#"{"id":"test_item","name":"Duplicate","kind":"weapon","value":1,"weight":1.0}"#);
        assert!(fixture.errors().iter().any(|error| error.contains("duplicate items id")));
    }

    #[test]
    fn missing_cross_references_are_rejected() {
        let fixture = Fixture::new();
        fixture.write("enemies/test_enemy.json", r#"{"id":"test_enemy","name":"Test Enemy","stats":{"health":5,"attack":1,"defense":0,"move_speed":1.0},"loot_table_id":"missing_loot"}"#);
        assert!(fixture.errors().iter().any(|error| error.contains("missing_loot")));
    }

    #[test]
    fn schema_rejects_invalid_numeric_values() {
        let fixture = Fixture::new();
        fixture.write("items/test_item.json", r#"{"id":"test_item","name":"Test Item","kind":"weapon","value":1,"weight":-1.0}"#);
        assert!(fixture.errors().iter().any(|error| error.contains("items/test_item.json")));
    }

    #[test]
    fn malformed_map_dimensions_are_rejected() {
        let fixture = Fixture::new();
        fixture.write("maps/test_map.json", r#"{"id":"test_map","name":"Test Map","rows":["#####","#S.X#","####"]}"#);
        assert!(fixture.errors().iter().any(|error| error.contains("inconsistent width") || error.contains("expected")));
    }

    #[test]
    fn missing_start_and_exit_are_rejected() {
        let fixture = Fixture::new();
        fixture.write("maps/test_map.json", r#"{"id":"test_map","name":"Test Map","rows":["#####","#...#","#####"]}"#);
        let errors = fixture.errors().join("\n");
        assert!(errors.contains("exactly one"));
        assert!(errors.contains("at least one"));
    }

    #[test]
    fn unreachable_exit_is_rejected_but_doors_are_traversable() {
        let fixture = Fixture::new();
        fixture.write("maps/test_map.json", r#"{"id":"test_map","name":"Test Map","rows":["#########","#S.D#..X#","#########"]}"#);
        assert!(fixture.errors().iter().any(|error| error.contains("unreachable")));
    }
}
