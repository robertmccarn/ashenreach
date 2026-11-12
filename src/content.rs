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
        let mut result = Self {
            root: root.clone(),
            ..Self::default()
        };
        let mut errors = Vec::new();
        result.items = load_directory(&root, "items", &mut errors);
        result.enemies = load_directory(&root, "enemies", &mut errors);
        result.loot_tables = load_directory(&root, "loot_tables", &mut errors);
        result.encounters = load_directory(&root, "encounters", &mut errors);
        result.maps = load_directory(&root, "maps", &mut errors);
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

fn load_directory<T: for<'a> Deserialize<'a>>(
    root: &Path,
    kind: &str,
    errors: &mut Vec<String>,
) -> HashMap<String, T>
where
    T: HasId,
{
    let directory = root.join(kind);
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
        match serde_json::from_str::<T>(&text) {
            Ok(value) => {
                let id = value.id().to_owned();
                if !valid_id(&id) {
                    errors.push(format!(
                        "{}: id `{id}` must use lowercase letters, digits, `_`, or `-`",
                        path.display()
                    ));
                } else if values.insert(id.clone(), value).is_some() {
                    errors.push(format!("{}: duplicate {kind} id `{id}`", path.display()));
                }
            }
            Err(error) => errors.push(format!(
                "{}:{}:{}: invalid {kind} JSON: {error}",
                path.display(),
                error.line(),
                error.column()
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
    #[test]
    fn shipped_content_is_valid() {
        assert!(ContentManager::load("content").is_ok());
    }
}
