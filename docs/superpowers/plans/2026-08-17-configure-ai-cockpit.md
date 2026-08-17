# Configure AI Cockpit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move ORG-X from installed-only AI Cockpit adoption to a confirmed adopter configuration that permits ordinary governed Work Items.

**Architecture:** Keep AI Cockpit as a governance layer. Confirm only facts observed by Doctor, explicitly approve `src/**` and `tests/**` as the project boundaries, bind quality checks to existing Rust Make targets, and expose the same checks through a full-history GitHub Actions workflow. Do not alter ORG-X production modules or business documents.

**Tech Stack:** Rust stable, Cargo, GNU Make, Python 3.11+, YAML, GitHub Actions.

## Global Constraints

- `repositoryRole` is `adopted` in the confirmed Project Profile.
- The calibration level is `lite`, selected with explicit human authorization.
- Approved production and test roots are `src/**` and `tests/**`.
- `Makefile.ai.stack` commands must be real executable project commands.
- CI must fetch full Git history and pass the pull request base SHA to `make check-ai-pr`.
- No secrets, credentials, machine-specific paths, Rust production behavior, product documents, or trading decision logic may change.

---

### Task 1: Confirm Project Profile and Coverage Boundary

**Files:**
- Create: `.ai/project_profile.yaml`
- Modify: `.ai/guards/coverage_policy.yaml`
- Test: `make check-ai-project-profile`
- Test: `make check-ai-guard-calibration`
- Test: `make check-ai-adoption-ready`

**Interfaces:**
- Consumes: `target/ai_project_doctor_report.json`, `.ai/calibration/profiles.yaml`, and the existing Coverage Guard.
- Produces: an approved Profile with `repositoryRole: adopted`, explicit `src/**` and `tests/**` boundaries, and `adoptionReviewed: true`.

- [ ] **Step 1: Write the confirmed Profile**

  ```yaml
  version: 1
  repositoryRole: adopted
  calibrationProfile:
    level: lite
    selectedBy: human
    selectedAt: "2026-08-17T11:07:37+09:00"
    reasons:
      - "Initial ORG-X adopter calibration uses explicit Rust source and test roots."
    requiredControls:
      - source_paths
      - test_paths
      - generated_paths
      - protected_paths
      - quality_command
      - default_branch
      - project_owner
      - reviewer
      - major_unknowns
    deferredControls:
      - file_ownership
      - scenario_coverage
      - destructive_change_policy
      - dependency_policy
      - ci_policy
      - public_api_policy
      - lifecycle_policy
      - delegated_evidence
      - reviewer_owner_separation
      - external_identity_evidence
      - release_evidence
      - sbom
      - provenance
      - signed_tag_policy
      - branch_protection_evidence
      - audit_retention
      - incident_exception_policy
  detectedFacts:
    languages: ["python", "rust"]
    frameworks: []
    buildSystems: ["cargo"]
    infrastructure: []
  suggestedBoundaries:
    productionRoots: ["src/**"]
    featureRoots: ["src/**"]
    testRoots: ["tests/**"]
    generatedPaths: []
    criticalPaths: []
  approvedBoundaries:
    productionRoots: ["src/**"]
    featureRoots: ["src/**"]
    testRoots: ["tests/**"]
    generatedPaths: []
    criticalPaths: []
  reviewRequirements: ["quality"]
  unknowns: []
  evidence:
    - "Cargo.toml"
    - "rust-toolchain.toml"
    - "target/ai_project_doctor_report.json"
  approval:
    reviewed: true
    reviewedBy: "user-authorized"
    reason: "The user authorized execution and the approved boundaries match the observed Rust repository layout."
  ```

- [ ] **Step 2: Mark the existing Coverage paths reviewed**

  Change only `adoptionReviewed: false` to `adoptionReviewed: true`; retain the existing include and exclude patterns because the installed runtime owns the broader governance-script coverage set.

- [ ] **Step 3: Run the Profile and readiness checks**

  Run:

  ```bash
  make check-ai-project-profile
  make check-ai-guard-calibration
  make check-ai-adoption-ready
  ```

  Expected: all three commands exit 0 and report no missing Profile, Coverage, or CI configuration (CI checks are completed in Task 3).

### Task 2: Bind Project Quality Commands

**Files:**
- Modify: `Makefile.ai.stack`
- Test: `make ai-cockpit-project-format-check`
- Test: `make ai-cockpit-project-lint`
- Test: `make ai-cockpit-project-test`

**Interfaces:**
- Consumes: existing `Makefile` targets and `rust-toolchain.toml` components.
- Produces: real executable commands used by `ai-cockpit-quality`.

- [ ] **Step 1: Set exact Rust commands**

  ```make
  PROJECT_FORMAT_CHECK = cargo fmt --all -- --check
  PROJECT_TEST = cargo test --all
  PROJECT_LINT = cargo clippy --all-targets --all-features -- -D warnings
  ```

- [ ] **Step 2: Run each configured command**

  Run the three Make targets above. Expected: formatter, clippy, and all Rust tests exit 0.

### Task 3: Add Hosted Governance Checks and Adopter Entrypoints

**Files:**
- Create: `.github/workflows/ai-cockpit.yml`
- Create: `.github/CODEOWNERS`
- Create: `SECURITY.md`
- Test: `make ai-cockpit-quality GOVERNANCE_PROFILE=strict`
- Test: `make check-ai-adoption-ready`

**Interfaces:**
- Consumes: `make ai-cockpit-quality`, `make check-ai-pr`, and the repository owner `@xinglun`.
- Produces: pull-request quality jobs with full history, an owner rule, and private security-reporting guidance.

- [ ] **Step 1: Add the workflow**

  ```yaml
  name: AI Cockpit

  on:
    pull_request:
    push:
      branches: [main]

  permissions:
    contents: read

  jobs:
    ai-cockpit-quality:
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v4
          with:
            fetch-depth: 0
        - uses: dtolnay/rust-toolchain@stable
        - run: make ai-cockpit-quality GOVERNANCE_PROFILE=strict

    check-ai-pr:
      if: github.event_name == 'pull_request'
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v4
          with:
            fetch-depth: 0
        - uses: dtolnay/rust-toolchain@stable
        - run: make check-ai-pr AI_BASE_COMMIT="${{ github.event.pull_request.base.sha }}"
  ```

- [ ] **Step 2: Add ownership and security entrypoints**

  `.github/CODEOWNERS` contains `* @xinglun`.

  `SECURITY.md` explains that security issues must use GitHub private vulnerability reporting or another private channel to the repository owner; it must not contain template-only instructions.

- [ ] **Step 3: Run local readiness and quality checks**

  Run:

  ```bash
  make ai-cockpit-quality GOVERNANCE_PROFILE=strict
  make check-ai-adoption-ready
  ```

  Expected: quality completes and readiness reports the configured adopter state. Hosted check results are collected only after pushing the PR.

### Task 4: Finish and Deliver the Configuration Work Item

**Files:**
- Modify: `.ai/work-items/active/configure_ai_cockpit.summary.json`
- Generate: `.ai/cockpit/current_status.md`, Task Outcome, and Human Benefit Report
- Archive: `.ai/work-items/archive/2026/`

**Interfaces:**
- Consumes: all local command output and the hosted PR checks.
- Produces: an archived, PR-bound configuration record that allows the next `MODE=code` Work Item.

- [ ] **Step 1: Record before-edit checkpoint**

  ```bash
  make ai-checkpoint \
    CONTRACT=.ai/work-items/active/configure_ai_cockpit.contract.json \
    SUMMARY=.ai/work-items/active/configure_ai_cockpit.summary.json \
    STAGE=before_edit
  ```

- [ ] **Step 2: Run all required AI Cockpit checks and finish**

  ```bash
  make ai-finish TASK=configure_ai_cockpit REPORT_LANGUAGE=zh-CN
  ```

  Expected: finish produces `completed` or `completed_with_warnings` only with all required checks passing and no out-of-scope paths.

- [ ] **Step 3: Archive, commit, and run PR preflight**

  ```bash
  make ai-finish TASK=configure_ai_cockpit ARCHIVE=true REPORT_LANGUAGE=zh-CN
  git add .ai docs/superpowers/plans/2026-08-17-configure-ai-cockpit.md Makefile.ai.stack .github SECURITY.md
  git commit -m "chore: configure AI Cockpit adoption"
  make check-ai-pr AI_BASE_COMMIT=ff2e76ba8af9aafeee50a2c10b9094c404f08c98
  ```

- [ ] **Step 4: Push, merge, close, and prove readiness**

  Push `codex/configure-ai-cockpit`, wait for both hosted jobs, merge without provider-side branch deletion, then run:

  ```bash
  make ai-close-work-item TASK=configure_ai_cockpit
  make check-ai-adoption-ready
  ```

  Expected: the close receipt reports a synchronized clean base and the readiness check exits 0. Only then create a fresh `codex/wi-002` branch from the new `origin/main`.
