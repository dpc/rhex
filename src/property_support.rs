//! Read-only regression inputs and writable failure diagnostics for bounded tests.

use proptest::test_runner::{Config, FailurePersistence, PersistedSeed};
use std::io::Write;

/// Each property has its own embedded corpus, independent of runtime source paths.
#[derive(Clone, Debug, PartialEq)]
struct Regressions {
    /// Stable diagnostic filename prefix.
    name: &'static str,
    /// Reviewed seeds compiled from the source-filtered tree.
    seeds: &'static str,
}

impl FailurePersistence for Regressions {
    fn load_persisted_failures2(&self, _: Option<&'static str>) -> Vec<PersistedSeed> {
        self.seeds
            .lines()
            .filter_map(|line| {
                let seed = line.split('#').next().unwrap().trim();
                (!seed.is_empty()).then(|| seed.parse().expect("invalid committed regression seed"))
            })
            .collect()
    }

    fn save_persisted_failure2(
        &mut self,
        _: Option<&'static str>,
        seed: PersistedSeed,
        value: &dyn std::fmt::Debug,
    ) {
        // Never write alongside source (which may be a read-only Nix store).
        // Always print both fields, even if the diagnostic directory is unwritable.
        let diagnostic = format!("{seed} # shrinks to {value:?}\n");
        eprintln!("{} regression: {diagnostic}", self.name);
        let directory = std::env::var_os("RHEX_PROPTEST_DIAGNOSTICS")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = directory.join(format!(
            "rhex-{}-{}-{stamp}.txt",
            self.name,
            std::process::id()
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => match file.write_all(diagnostic.as_bytes()) {
                Ok(()) => eprintln!("Regression diagnostic: {}", path.display()),
                Err(error) => eprintln!("Could not write {}: {error}", path.display()),
            },
            Err(error) => eprintln!("Could not create {}: {error}", path.display()),
        }
    }

    fn box_clone(&self) -> Box<dyn FailurePersistence> {
        Box::new(self.clone())
    }

    fn eq(&self, other: &dyn FailurePersistence) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Keep ordinary runs bounded; PROPTEST_CASES opts into a larger local stress run.
pub(crate) fn config(name: &'static str, seeds: &'static str, cases: u32) -> Config {
    Config {
        cases: std::env::var("PROPTEST_CASES")
            .map(|value| value.parse().expect("PROPTEST_CASES must be an integer"))
            .unwrap_or(cases),
        max_shrink_iters: 4096,
        failure_persistence: Some(Box::new(Regressions { name, seeds })),
        ..Config::default()
    }
}
