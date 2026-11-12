use std::{
    env, fmt, fs,
    path::{Path, PathBuf},
};

/// Resolves the authored content directory without performing content parsing.
///
/// An explicit root is authoritative. With no explicit root, the resolver uses
/// `content/` below the supplied working directory. The executable location is
/// intentionally not searched because this project does not define a packaged
/// resource layout yet.
pub fn resolve_content_root(
    explicit_root: Option<&Path>,
    working_directory: &Path,
) -> Result<PathBuf, ContentRootError> {
    if let Some(root) = explicit_root {
        let candidate = absolutize(root, working_directory);
        return if is_directory(&candidate) {
            Ok(candidate)
        } else {
            Err(ContentRootError::new(
                "the explicitly supplied content root is missing or is not a directory",
                vec![candidate],
            ))
        };
    }

    let candidate = working_directory.join("content");
    if is_directory(&candidate) {
        Ok(candidate)
    } else {
        Err(ContentRootError::new(
            "no supported content root was found",
            vec![candidate],
        ))
    }
}

/// Resolves the default root using the process working directory.
pub fn resolve_default_content_root() -> Result<PathBuf, ContentRootError> {
    let working_directory = env::current_dir().map_err(|error| {
        ContentRootError::new(
            format!("cannot determine the process working directory: {error}"),
            Vec::new(),
        )
    })?;
    resolve_content_root(None, &working_directory)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentRootError {
    reason: String,
    attempted: Vec<PathBuf>,
}

impl ContentRootError {
    fn new(reason: impl Into<String>, attempted: Vec<PathBuf>) -> Self {
        Self {
            reason: reason.into(),
            attempted,
        }
    }

    pub fn attempted(&self) -> &[PathBuf] {
        &self.attempted
    }
}

impl fmt::Display for ContentRootError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.reason)?;
        if !self.attempted().is_empty() {
            write!(formatter, "; attempted: ")?;
            for (index, path) in self.attempted().iter().enumerate() {
                if index > 0 {
                    write!(formatter, ", ")?;
                }
                write!(formatter, "{}", path.display())?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for ContentRootError {}

fn absolutize(path: &Path, working_directory: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        working_directory.join(path)
    }
}

fn is_directory(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_dir())
}

#[cfg(test)]
mod tests {
    use super::resolve_content_root;
    use crate::content::ContentManager;
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

    struct Fixture {
        root: PathBuf,
        working_directory: PathBuf,
    }

    impl Fixture {
        fn new(with_default_content: bool) -> Self {
            let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "ashen-reach-b1-root-test-{}-{id}",
                std::process::id()
            ));
            let working_directory = root.join("work");
            fs::create_dir_all(&working_directory).unwrap();
            if with_default_content {
                fs::create_dir_all(working_directory.join("content")).unwrap();
            }
            Self {
                root,
                working_directory,
            }
        }

        fn explicit_root(&self) -> PathBuf {
            let root = self.root.join("explicit");
            fs::create_dir_all(&root).unwrap();
            root
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn default_working_directory_content_resolves() {
        let fixture = Fixture::new(true);
        let resolved = resolve_content_root(None, &fixture.working_directory).unwrap();
        assert_eq!(resolved, fixture.working_directory.join("content"));
    }

    #[test]
    fn explicit_absolute_root_resolves() {
        let fixture = Fixture::new(false);
        let explicit = fixture.explicit_root();
        assert_eq!(
            resolve_content_root(Some(&explicit), &fixture.working_directory),
            Ok(explicit)
        );
    }

    #[test]
    fn explicit_relative_root_is_resolved_from_the_supplied_working_directory() {
        let fixture = Fixture::new(false);
        fs::create_dir_all(fixture.working_directory.join("authored")).unwrap();
        let resolved =
            resolve_content_root(Some(Path::new("authored")), &fixture.working_directory).unwrap();
        assert_eq!(resolved, fixture.working_directory.join("authored"));
    }

    #[test]
    fn missing_explicit_root_does_not_fall_back_to_default() {
        let fixture = Fixture::new(true);
        let missing = fixture.root.join("missing");
        let error = resolve_content_root(Some(&missing), &fixture.working_directory).unwrap_err();
        assert_eq!(error.attempted(), &[missing]);
        assert!(error.to_string().contains("explicitly supplied"));
        assert!(!error.to_string().contains("content\\content"));
    }

    #[test]
    fn missing_default_root_reports_attempted_location() {
        let fixture = Fixture::new(false);
        let error = resolve_content_root(None, &fixture.working_directory).unwrap_err();
        assert_eq!(
            error.attempted(),
            &[fixture.working_directory.join("content")]
        );
        assert!(error.to_string().contains("attempted:"));
    }

    #[test]
    fn resolution_is_deterministic_when_other_candidates_exist() {
        let fixture = Fixture::new(true);
        fs::create_dir_all(fixture.root.join("content")).unwrap();
        let first = resolve_content_root(None, &fixture.working_directory).unwrap();
        let second = resolve_content_root(None, &fixture.working_directory).unwrap();
        assert_eq!(first, second);
        assert_eq!(first, fixture.working_directory.join("content"));
    }

    #[test]
    fn selected_root_is_passed_to_content_manager_without_fallback() {
        let fixture = Fixture::new(true);
        let explicit = fixture.explicit_root();
        fs::write(explicit.join("manifest.json"), "{}").unwrap();

        let resolved = resolve_content_root(Some(&explicit), &fixture.working_directory).unwrap();
        let errors = ContentManager::load(&resolved).unwrap_err();

        assert!(errors.iter().any(|error| error.contains("manifest.json")));
        assert!(errors.iter().any(|error| error.contains("format_version")));
    }
}
