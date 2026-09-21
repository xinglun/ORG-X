use std::fs;
use std::path::Path;

#[test]
fn all_github_actions_jobs_use_pinned_ubuntu_runner() {
    let workflow_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows");
    let mut workflow_count = 0;

    for entry in fs::read_dir(&workflow_dir).expect("workflow directory should exist") {
        let path = entry
            .expect("workflow directory entry should be readable")
            .path();
        let extension = path.extension().and_then(|value| value.to_str());
        if !matches!(extension, Some("yml") | Some("yaml")) {
            continue;
        }

        workflow_count += 1;
        let workflow = fs::read_to_string(&path).expect("workflow should be readable");
        assert!(
            !workflow.contains("ubuntu-latest"),
            "{} must not reintroduce ubuntu-latest",
            path.display()
        );

        for line in workflow.lines() {
            if line.trim_start().starts_with("runs-on:") {
                assert_eq!(
                    line.trim(),
                    "runs-on: ubuntu-24.04",
                    "{} must pin every job to ubuntu-24.04",
                    path.display()
                );
            }
        }
    }

    assert!(
        workflow_count > 0,
        "at least one GitHub Actions workflow is required"
    );
}
