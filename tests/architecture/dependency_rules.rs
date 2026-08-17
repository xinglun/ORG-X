use std::fs;
use std::path::{Path, PathBuf};

const CONTEXTS: &[&str] = &[
    "diffusion",
    "evidence",
    "ingestion",
    "organization",
    "productivity",
    "production_system",
    "ranking",
    "reporting",
    "transformation",
    "universe",
];

fn source_files_under(relative: &str) -> Vec<PathBuf> {
    fn collect(path: &Path, files: &mut Vec<PathBuf>) {
        let entries = fs::read_dir(path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));

        for entry in entries {
            let child = entry
                .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", path.display()))
                .path();
            if child.is_dir() {
                collect(&child, files);
            } else if child.extension().is_some_and(|extension| extension == "rs") {
                files.push(child);
            }
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let mut files = Vec::new();
    collect(&root, &mut files);
    files.sort();
    files
}

fn assert_sources_do_not_contain(relative: &str, forbidden: &[&str]) {
    let context_module = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(relative)
        .join("mod.rs");
    let is_context_root = CONTEXTS
        .iter()
        .any(|context| relative == format!("src/features/{context}"));
    let files: Vec<_> = source_files_under(relative)
        .into_iter()
        .filter(|path| !is_context_root || path != &context_module)
        .collect();
    assert!(!files.is_empty(), "no Rust sources found under {relative}");

    for path in files {
        let contents = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let lower_contents = contents.to_ascii_lowercase();

        for token in forbidden {
            let lower_token = token.to_ascii_lowercase();
            assert!(
                !lower_contents.contains(&lower_token),
                "{} contains forbidden architecture token `{token}`",
                path.display()
            );
        }
    }
}

#[test]
fn domain_does_not_depend_on_infrastructure() {
    for context in CONTEXTS {
        assert_sources_do_not_contain(
            &format!("src/features/{context}/domain"),
            &[
                "infrastructure",
                "interface",
                "reqwest",
                "sqlx",
                "llm",
                "provider",
            ],
        );
    }
}

#[test]
fn transformation_does_not_depend_on_llm() {
    assert_sources_do_not_contain(
        "src/features/transformation",
        &["llm", "provider", "infrastructure"],
    );
}

#[test]
fn ranking_does_not_depend_on_external_provider() {
    assert_sources_do_not_contain(
        "src/features/ranking",
        &["provider", "reqwest", "sqlx", "renderer", "infrastructure"],
    );
}

#[test]
fn production_system_does_not_depend_on_renderer() {
    assert_sources_do_not_contain(
        "src/features/production_system",
        &["renderer", "reporting", "interface"],
    );
}

#[test]
fn evidence_domain_is_provider_agnostic() {
    assert_sources_do_not_contain(
        "src/features/evidence/domain",
        &["provider", "sec", "news", "fundamental", "json"],
    );
}
