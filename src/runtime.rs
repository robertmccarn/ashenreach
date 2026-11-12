use crate::{content::ContentManager, game::Game};

/// The active authored content and runtime state are replaced as one unit.
#[derive(Clone, Debug)]
pub struct RuntimeSession {
    pub content: ContentManager,
    pub game: Game,
}

impl RuntimeSession {
    pub fn load(root: impl Into<std::path::PathBuf>, map_id: &str) -> Result<Self, Vec<String>> {
        let content = ContentManager::load(root)?;
        Self::from_content(content, map_id)
    }

    pub fn reload(&mut self, map_id: &str) -> Result<(), Vec<String>> {
        let candidate_content = ContentManager::load(self.content.root().to_owned())?;
        let candidate = Self::from_content(candidate_content, map_id)?;
        *self = candidate;
        Ok(())
    }

    pub fn restart(&mut self, map_id: &str) -> Result<(), String> {
        let candidate_game = self.construct_game(map_id)?;
        self.game = candidate_game;
        Ok(())
    }

    fn from_content(content: ContentManager, map_id: &str) -> Result<Self, Vec<String>> {
        let game = content
            .map(map_id)
            .and_then(Game::from_map)
            .map_err(|error| vec![format!("runtime map `{map_id}`: {error}")])?;
        Ok(Self { content, game })
    }

    fn construct_game(&self, map_id: &str) -> Result<Game, String> {
        self.content.map(map_id).and_then(Game::from_map)
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeSession;
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "ashen-reach-b2-runtime-test-{}-{id}",
                std::process::id()
            ));
            copy_directory(
                &Path::new(env!("CARGO_MANIFEST_DIR")).join("content"),
                &root,
            )
            .unwrap();
            Self { root }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn copy_directory(source: &Path, destination: &Path) -> std::io::Result<()> {
        fs::create_dir_all(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let target = destination.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_directory(&entry.path(), &target)?;
            } else {
                fs::copy(entry.path(), target)?;
            }
        }
        Ok(())
    }

    fn write_map(root: &Path, name: &str) {
        let map = format!(
            "{{\"id\":\"prototype_dungeon\",\"name\":\"{name}\",\"rows\":[\"#####\",\"#S.X#\",\"#####\"]}}"
        );
        fs::write(root.join("maps/prototype_dungeon.json"), map).unwrap();
    }

    fn assert_representative_content_unchanged(
        current: &RuntimeSession,
        previous: &RuntimeSession,
    ) {
        for (kind, id, current_definition, previous_definition) in [
            (
                "map",
                "prototype_dungeon",
                format!("{:?}", current.content.maps["prototype_dungeon"]),
                format!("{:?}", previous.content.maps["prototype_dungeon"]),
            ),
            (
                "item",
                "iron_sword",
                format!("{:?}", current.content.items["iron_sword"]),
                format!("{:?}", previous.content.items["iron_sword"]),
            ),
            (
                "enemy",
                "crypt_rat",
                format!("{:?}", current.content.enemies["crypt_rat"]),
                format!("{:?}", previous.content.enemies["crypt_rat"]),
            ),
            (
                "encounter",
                "gatehouse_patrol",
                format!("{:?}", current.content.encounters["gatehouse_patrol"]),
                format!("{:?}", previous.content.encounters["gatehouse_patrol"]),
            ),
        ] {
            assert_eq!(
                current_definition, previous_definition,
                "{kind} `{id}` changed while replacement failed"
            );
        }
    }

    #[test]
    fn successful_reload_replaces_content_and_runtime_together() {
        let fixture = Fixture::new();
        let mut session = RuntimeSession::load(&fixture.root, "prototype_dungeon").unwrap();
        let original = session.game.clone();
        write_map(&fixture.root, "Reloaded");

        session.reload("prototype_dungeon").unwrap();

        assert_eq!(session.content.maps["prototype_dungeon"].name, "Reloaded");
        assert_ne!(session.game, original);
        assert_eq!(session.game.world.dimensions(), (5, 3));
    }

    #[test]
    fn invalid_content_preserves_the_previous_session() {
        let fixture = Fixture::new();
        let mut session = RuntimeSession::load(&fixture.root, "prototype_dungeon").unwrap();
        let previous = session.clone();
        fs::write(fixture.root.join("manifest.json"), "{}").unwrap();

        assert!(session.reload("prototype_dungeon").is_err());
        assert_eq!(session.game, previous.game);
        assert_eq!(session.content.root(), previous.content.root());
        assert_representative_content_unchanged(&session, &previous);
    }

    #[test]
    fn valid_content_missing_runtime_map_preserves_the_previous_session() {
        let fixture = Fixture::new();
        let mut session = RuntimeSession::load(&fixture.root, "prototype_dungeon").unwrap();
        let previous = session.clone();
        fs::remove_file(fixture.root.join("maps/prototype_dungeon.json")).unwrap();

        let errors = session.reload("prototype_dungeon").unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("prototype_dungeon"))
        );
        assert_eq!(session.game, previous.game);
        assert_eq!(session.content.root(), previous.content.root());
        assert_representative_content_unchanged(&session, &previous);
    }

    #[test]
    fn restart_uses_the_current_content_source() {
        let fixture = Fixture::new();
        let mut session = RuntimeSession::load(&fixture.root, "prototype_dungeon").unwrap();
        session.game.elapsed_seconds = 7.0;

        session.restart("prototype_dungeon").unwrap();

        assert_eq!(session.game.elapsed_seconds, 0.0);
        assert_eq!(session.content.root(), fixture.root.as_path());
    }

    #[test]
    fn failed_restart_preserves_the_previous_runtime_state() {
        let fixture = Fixture::new();
        let mut session = RuntimeSession::load(&fixture.root, "prototype_dungeon").unwrap();
        let previous = session.clone();
        assert!(session.restart("missing_runtime_map").is_err());
        assert_eq!(session.game, previous.game);
        assert_eq!(session.content.root(), previous.content.root());
        assert_representative_content_unchanged(&session, &previous);
    }

    #[test]
    fn repeated_reloads_do_not_mix_runtime_state() {
        let fixture = Fixture::new();
        let mut session = RuntimeSession::load(&fixture.root, "prototype_dungeon").unwrap();
        for name in ["First", "Second", "Third"] {
            write_map(&fixture.root, name);
            session.reload("prototype_dungeon").unwrap();
            assert_eq!(session.content.maps["prototype_dungeon"].name, name);
            assert_eq!(session.game.world.dimensions(), (5, 3));
        }
    }
}
