use std::fs;
use std::path::Path;

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
    "weekly_radar",
];

const LAYERS: &[&str] = &[
    "domain",
    "application",
    "infrastructure",
    "interface",
    "acl",
];

#[test]
fn all_bounded_contexts_have_five_layers() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));

    for context in CONTEXTS {
        let context_root = manifest.join("src/features").join(context);
        assert!(
            context_root.is_dir(),
            "missing bounded context directory: {}",
            context_root.display()
        );

        for layer in LAYERS {
            let layer_root = context_root.join(layer);
            assert!(
                layer_root.is_dir(),
                "missing {layer} layer for {context}: {}",
                layer_root.display()
            );
            assert!(
                layer_root.join("mod.rs").is_file(),
                "missing {layer}/mod.rs for {context}"
            );
        }
    }
}

#[test]
fn root_exports_architecture_roots_without_implementation_leaks() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lib = fs::read_to_string(manifest.join("src/lib.rs")).expect("cannot read src/lib.rs");

    assert!(lib.contains("pub mod features;"));
    assert!(lib.contains("pub mod shared;"));
    assert!(!lib.contains("mod main"));
    assert!(!lib.to_ascii_lowercase().contains("provider"));
}
