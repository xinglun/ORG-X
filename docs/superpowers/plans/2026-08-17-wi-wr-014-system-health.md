# WI-WR-014 System Health Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add explicit, typed System Health facts to the provider-agnostic Weekly Radar publication boundary without deriving health, Stage, or Ranking.

**Architecture:** Keep all health value objects and collection invariants in `weekly_radar::domain::system_health`, using only standard-library types. Extend `WeeklyRadarPublication` with an optional, single-assignment `SystemHealth` section; leave the existing publisher port and all external adapters unchanged.

**Tech Stack:** Rust 2021, Cargo unit/integration tests, existing Weekly Radar Domain, AI Cockpit Make targets, Markdown governance documents, and JSON reference-impact evidence. No new dependency.

## Global Constraints

- HealthStatus, Freshness, coverage values, degraded companies, source coverage, and extraction failures are supplied facts; constructors validate shape but do not infer health.
- Domain has no Telegram, HTTP, Secret, network, database, Scheduler, persistence, rendering, delivery, retry, receipt, or provider imports.
- Do not derive or change Stage, Ranking, Top5, Threshold Distance, Rising, Dropped, trading, or capital-action behavior.
- Do not modify shared architecture tests, global coverage policy, project-wide Make targets, or unrelated bounded contexts.
- Preserve supplied order and reject duplicate identities without replacing earlier facts.
- Required lifecycle is strict AI Cockpit Finish, Archive, and local commit only; do not push, open a PR, merge, or close.

---

### Task 1: Add the explicit System Health value objects and aggregate

**Files:**
- Create: `src/features/weekly_radar/domain/system_health.rs`
- Create: `src/features/weekly_radar/domain/system_health_test.rs`
- Modify: `src/features/weekly_radar/domain/mod.rs`
- Test: `tests/system_health_test.rs`

**Interfaces:**
- Consumes: no other feature module; accepts primitive counts, percentages, opaque text references, and explicit enum values.
- Produces: `HealthStatus`, `Freshness`, `CoveragePercentage`, `EvidenceCoverage`, `CompanyReference`, `SourceReference`, `FailureId`, `Reason`, `DegradedCompany`, `ExtractionFailure`, `SystemHealth`, and `SystemHealthDomainError`.

- [ ] **Step 1: Write the failing companion test for explicit facts**

Create `tests/system_health_test.rs` with this test before adding the production module:

```rust
use org_x::features::weekly_radar::domain::system_health::{
    CompanyReference, EvidenceCoverage, Freshness, HealthStatus, SystemHealth,
};

#[test]
fn supplied_status_coverage_and_freshness_are_retained_without_inference() {
    let coverage = EvidenceCoverage::new(1, 2, 99).unwrap();
    let health = SystemHealth::new(HealthStatus::Healthy, coverage.clone(), Freshness::Stale);

    assert_eq!(health.status(), HealthStatus::Healthy);
    assert_eq!(health.evidence_coverage(), &coverage);
    assert_eq!(health.freshness(), Freshness::Stale);
    assert_eq!(CompanyReference::new("company-a").unwrap().as_str(), "company-a");
}
```

- [ ] **Step 2: Run the test to verify the expected missing-feature failure**

Run: `cargo test --test system_health_test`

Expected: FAIL at compilation because `weekly_radar::domain::system_health` is not yet registered. This is the intended RED state, not a test typo.

- [ ] **Step 3: Implement the minimal public model and register it**

Add `pub mod system_health;` to `src/features/weekly_radar/domain/mod.rs` and create `system_health.rs` with the following API shape:

```rust
use std::fmt;

#[cfg(test)]
#[path = "system_health_test.rs"]
mod module_tests;

fn non_empty(field: &'static str, value: impl Into<String>) -> Result<String, SystemHealthDomainError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(SystemHealthDomainError::EmptyValue { field });
    }
    Ok(value)
}

macro_rules! text_value {
    ($name:ident, $field:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, SystemHealthDomainError> {
                Ok(Self(non_empty($field, value)?))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

text_value!(CompanyReference, "company reference");
text_value!(SourceReference, "source reference");
text_value!(FailureId, "failure id");
text_value!(Reason, "reason");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthStatus { Healthy, Degraded, Unavailable, Unknown }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Freshness { Current, Aging, Stale, Unknown }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoveragePercentage(u8);

impl CoveragePercentage {
    pub fn new(value: u8) -> Result<Self, SystemHealthDomainError> {
        if value > 100 {
            return Err(SystemHealthDomainError::InvalidPercentage { value });
        }
        Ok(Self(value))
    }

    pub const fn value(self) -> u8 { self.0 }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceCoverage {
    available: u32,
    expected: u32,
    percentage: CoveragePercentage,
}

impl EvidenceCoverage {
    pub fn new(available: u32, expected: u32, percentage: u8) -> Result<Self, SystemHealthDomainError> {
        Ok(Self { available, expected, percentage: CoveragePercentage::new(percentage)? })
    }

    pub const fn available(&self) -> u32 { self.available }
    pub const fn expected(&self) -> u32 { self.expected }
    pub const fn percentage(&self) -> CoveragePercentage { self.percentage }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DegradedCompany { company: CompanyReference, reason: Reason }

impl DegradedCompany {
    pub fn new(company: CompanyReference, reason: Reason) -> Self { Self { company, reason } }
    pub fn company(&self) -> &CompanyReference { &self.company }
    pub fn reason(&self) -> &Reason { &self.reason }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceCoverage {
    source: SourceReference,
    available: u32,
    expected: u32,
    percentage: CoveragePercentage,
}

impl SourceCoverage {
    pub fn new(source: SourceReference, available: u32, expected: u32, percentage: u8) -> Result<Self, SystemHealthDomainError> {
        Ok(Self { source, available, expected, percentage: CoveragePercentage::new(percentage)? })
    }
    pub fn source(&self) -> &SourceReference { &self.source }
    pub const fn available(&self) -> u32 { self.available }
    pub const fn expected(&self) -> u32 { self.expected }
    pub const fn percentage(&self) -> CoveragePercentage { self.percentage }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractionFailure { id: FailureId, source: SourceReference, reason: Reason }

impl ExtractionFailure {
    pub fn new(id: FailureId, source: SourceReference, reason: Reason) -> Self { Self { id, source, reason } }
    pub fn id(&self) -> &FailureId { &self.id }
    pub fn source(&self) -> &SourceReference { &self.source }
    pub fn reason(&self) -> &Reason { &self.reason }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SystemHealthDomainError {
    EmptyValue { field: &'static str },
    InvalidPercentage { value: u8 },
    DuplicateIdentity { entity: &'static str, id: String },
}

impl fmt::Display for SystemHealthDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::InvalidPercentage { value } => write!(formatter, "coverage percentage {value} exceeds 100"),
            Self::DuplicateIdentity { entity, id } => write!(formatter, "duplicate {entity} identity {id}"),
        }
    }
}

impl std::error::Error for SystemHealthDomainError {}
```

Then add `SystemHealth` with `new`, `status`, `evidence_coverage`, `freshness`, empty ordered vectors, `degraded_companies`, `source_coverage`, and `extraction_failures` accessors. Add `add_degraded_company`, `add_source_coverage`, and `add_extraction_failure`; each compares the relevant opaque identity and returns `DuplicateIdentity` before mutating.

- [ ] **Step 4: Run the focused test to verify GREEN**

Run: `cargo test --test system_health_test`

Expected: PASS with the explicit Healthy/Stale facts retained exactly.

- [ ] **Step 5: Add module-local validation and collection tests**

In `src/features/weekly_radar/domain/system_health_test.rs`, add tests for blank `CompanyReference`, invalid percentage 101, source order, duplicate source rejection without mutation, duplicate company rejection, duplicate failure rejection, and `HealthStatus::Degraded` remaining unchanged when its coverage is full.

- [ ] **Step 6: Run all library and companion tests**

Run: `cargo test --lib system_health` and `cargo test --test system_health_test`

Expected: PASS with no warnings or failures.

### Task 2: Attach System Health to Weekly Radar publication

**Files:**
- Modify: `src/features/weekly_radar/domain/mod.rs`
- Create: `tests/weekly_radar_system_health.rs`

**Interfaces:**
- Consumes: `SystemHealth` from `weekly_radar::domain::system_health` and the existing `WeeklyRadarSnapshot`/`WeeklyRadarPublication`.
- Produces: `WeeklyRadarPublication::set_system_health` and `WeeklyRadarPublication::system_health` without changing `WeeklyRadarPublisher`.

- [ ] **Step 1: Write the failing publication integration test**

Create `tests/weekly_radar_system_health.rs` with a snapshot helper and this behavior:

```rust
use org_x::features::weekly_radar::domain::system_health::{
    EvidenceCoverage, Freshness, HealthStatus, SystemHealth,
};
use org_x::features::weekly_radar::domain::{
    AsOf, EvidenceCutoff, ModelVersion, ScoringVersion, SnapshotId,
    UniverseSnapshotId, WeeklyRadarDomainError, WeeklyRadarPublication, WeeklyRadarSnapshot,
};

fn publication() -> WeeklyRadarPublication {
    WeeklyRadarPublication::new(
        WeeklyRadarSnapshot::new(
            SnapshotId::new("snapshot-health").unwrap(),
            AsOf::new("2026-08-16").unwrap(),
            UniverseSnapshotId::new("universe-health").unwrap(),
            EvidenceCutoff::new("cutoff-health").unwrap(),
            ModelVersion::new("model-health").unwrap(),
            ScoringVersion::new("score-health").unwrap(),
        )
        .unwrap(),
    )
}

#[test]
fn publication_retains_one_supplied_health_section() {
    let mut publication = publication();
    let health = SystemHealth::new(
        HealthStatus::Degraded,
        EvidenceCoverage::new(2, 3, 88).unwrap(),
        Freshness::Aging,
    );
    publication.set_system_health(health.clone()).unwrap();

    assert_eq!(publication.system_health(), Some(&health));
    assert_eq!(publication.snapshot_id().as_str(), "snapshot-health");
}

#[test]
fn publication_rejects_replacing_the_health_section() {
    let mut publication = publication();
    let first = SystemHealth::new(
        HealthStatus::Healthy,
        EvidenceCoverage::new(3, 3, 100).unwrap(),
        Freshness::Current,
    );
    let second = SystemHealth::new(
        HealthStatus::Unavailable,
        EvidenceCoverage::new(0, 3, 0).unwrap(),
        Freshness::Unknown,
    );
    publication.set_system_health(first.clone()).unwrap();

    assert_eq!(
        publication.set_system_health(second),
        Err(WeeklyRadarDomainError::DuplicateIdentity {
            entity: "system health",
            id: "snapshot-health".to_owned(),
        })
    );
    assert_eq!(publication.system_health(), Some(&first));
}
```

- [ ] **Step 2: Run the test to verify the expected RED state**

Run: `cargo test --test weekly_radar_system_health`

Expected: FAIL to compile because `WeeklyRadarPublication` has no System Health setter/accessor yet.

- [ ] **Step 3: Implement the publication attachment**

In `src/features/weekly_radar/domain/mod.rs`, import `SystemHealth`, add `system_health: Option<SystemHealth>` to `WeeklyRadarPublication`, initialize it to `None`, and add:

```rust
pub fn set_system_health(
    &mut self,
    system_health: SystemHealth,
) -> Result<(), WeeklyRadarDomainError> {
    if self.system_health.is_some() {
        return Err(WeeklyRadarDomainError::DuplicateIdentity {
            entity: "system health",
            id: self.snapshot.id().as_str().to_owned(),
        });
    }
    self.system_health = Some(system_health);
    Ok(())
}

pub fn system_health(&self) -> Option<&SystemHealth> {
    self.system_health.as_ref()
}
```

- [ ] **Step 4: Run the public publication tests to verify GREEN**

Run: `cargo test --test weekly_radar_system_health && cargo test --test weekly_radar_contract`

Expected: PASS; the existing publisher port remains unchanged and publication facts retain their previous order/identity behavior.

- [ ] **Step 5: Confirm the provider-agnostic source boundary**

Run: `rg -n -i "telegram|http|secret|api[_ -]?key|credential|scheduler|database|network" src/features/weekly_radar/domain src/features/weekly_radar/application`

Expected: no provider or credential implementation references in the changed Domain/Application source. Any existing unrelated documentation match is outside the changed source boundary and is not modified.

### Task 3: Record reference impact and complete documentation alignment

**Files:**
- Create: `.ai/evidence/reference-impact/wi-wr-014-system-health.json`
- Modify: `.ai/work-items/active/wi-wr-014.summary.json`
- Modify: `docs/superpowers/specs/2026-08-17-wi-wr-014-system-health.md`
- Modify: `docs/superpowers/plans/2026-08-17-wi-wr-014-system-health.md`

**Interfaces:**
- Consumes: Contract scope, final diff, focused test evidence, and user authorization recorded in the Contract.
- Produces: machine-readable reference-impact evidence and a Summary ready for strict AI Cockpit Finish.

- [ ] **Step 1: Write reference-impact evidence for the new Domain boundary**

Create `.ai/evidence/reference-impact/wi-wr-014-system-health.json` with this schema-valid evidence record:

```json
{
  "version": 1,
  "requestedText": "Add typed System Health facts to the provider-agnostic Weekly Radar publication boundary.",
  "target": {
    "type": "file",
    "path": "src/features/weekly_radar/domain/system_health.rs",
    "operation": "change_signature"
  },
  "referenceAnalysis": {
    "staticReferences": [
      "src/features/weekly_radar/domain/mod.rs",
      "tests/weekly_radar_system_health.rs",
      "tests/system_health_test.rs"
    ],
    "testReferences": [
      "src/features/weekly_radar/domain/system_health_test.rs",
      "tests/weekly_radar_system_health.rs",
      "tests/system_health_test.rs"
    ],
    "documentationReferences": [
      "docs/superpowers/specs/2026-08-17-wi-wr-014-system-health.md",
      "docs/superpowers/plans/2026-08-17-wi-wr-014-system-health.md"
    ],
    "dynamicReferences": {
      "status": "proven_absent",
      "evidence": [
        "The Domain module stores supplied in-memory facts and has no runtime loading or dynamic reference mechanism."
      ]
    },
    "externalConsumers": {
      "status": "proven_absent",
      "evidence": [
        "The only changed consumer is the existing repository-local WeeklyRadarPublication boundary; no external adapter is added."
      ]
    },
    "monitoringReferences": {
      "status": "proven_absent",
      "evidence": [
        "The Work Item adds no telemetry, monitoring, delivery, HTTP, or provider integration."
      ]
    }
  },
  "governanceEvidence": {
    "contractDeclared": true,
    "acceptanceDeclared": true,
    "destructiveChangeAllowed": false,
    "evidence": [
      ".ai/work-items/active/wi-wr-014.contract.json",
      "The Contract declares an additive provider-agnostic Domain boundary and explicitly excludes provider, architecture-test, and global-policy changes."
    ]
  }
}
```

- [ ] **Step 2: Update the Summary with changed files and executable evidence**

Record every production, test, documentation, reference-impact, Contract, generated, and archive path with an evidence-bound reason. Mark all five Contract scenarios `verified` only after their focused commands pass; record residual boundaries as future renderer/provider work rather than as completed benefits.

- [ ] **Step 3: Run documentation and Contract self-review**

Run: `rg -n "TBD|TODO|placeholder|replace-with|implemented|updated|as needed|if needed|etc\." docs/superpowers/specs/2026-08-17-wi-wr-014-system-health.md docs/superpowers/plans/2026-08-17-wi-wr-014-system-health.md .ai/work-items/active/wi-wr-014.contract.json`

Expected: no placeholder or vague acceptance text; spec, plan, Contract, and code names match exactly.

### Task 4: Run TDD regression, quality, and AI Cockpit Finish

**Files:**
- Modify: `.ai/work-items/active/wi-wr-014.summary.json`
- Generated by Make: `.ai/cockpit/current_status.md`, `.ai/cockpit/task_report.json`, `.ai/cockpit/task_report.md`, `.ai/work-items/active/wi-wr-014.outcome.json`, `.ai/work-items/active/wi-wr-014.outcome.md`

**Interfaces:**
- Consumes: final implementation and Summary evidence.
- Produces: strict Work Item Outcome and Finish evidence; no provider-side mutation.

- [ ] **Step 1: Run the focused TDD and formatting checks**

Run: `cargo test --lib system_health`, `cargo test --test system_health_test`, `cargo test --test weekly_radar_system_health`, `cargo fmt --all -- --check`

Expected: all focused tests pass and formatting check exits 0.

- [ ] **Step 2: Run full project quality checks**

Run: `make check`

Expected: format, clippy with warnings denied, and all unit/integration tests pass.

- [ ] **Step 3: Record the before-finish checkpoint**

Run: `make ai-checkpoint CONTRACT=.ai/work-items/active/wi-wr-014.contract.json SUMMARY=.ai/work-items/active/wi-wr-014.summary.json STAGE=before_finish`

Expected: checkpoint records the final Contract hash, zero unknowns, changed-file ownership, and the next action as Finish.

- [ ] **Step 4: Run strict AI Cockpit Finish**

Run: `make ai-finish TASK=wi-wr-014 REPORT_LANGUAGE=zh-CN`

Expected: all 16 Contract verification checks, project quality, Summary, scenario coverage, scope, guards, status, and generated Outcome evidence pass. If coverage reports an unrecognized integration test, add only the declared same-stem module test and rerun the guard without changing shared policy.

### Task 5: Archive and create the local handoff commit

**Files:**
- Generated/archive: `.ai/work-items/archive/**`, `.ai/cockpit/current_status.md`, `.ai/cockpit/task_report.json`, `.ai/cockpit/task_report.md`
- No changes: remote branches, PRs, merge state, or closure state

**Interfaces:**
- Consumes: successful strict Finish and active Work Item evidence.
- Produces: archived WI-WR-014 evidence and a local branch commit SHA.

- [ ] **Step 1: Verify active Finish evidence before archive**

Run: `git status --short` and `make check-ai-pr AI_BASE_COMMIT=ba346c2bbd538d2f734951ca05f3ad0322979cfa`

Expected: all changed paths are Contract-owned, the active Outcome/Summary pair is valid, and no out-of-scope path is present.

- [ ] **Step 2: Archive the Work Item without closing it**

Run: `make archive-work-item CONTRACT=.ai/work-items/active/wi-wr-014.contract.json`

Expected: Contract, Summary, Outcome, Start Receipt, generated cockpit reports, and archive manifest move into the immutable archive evidence set; no push, PR, merge, or close command runs.

- [ ] **Step 3: Run archive integrity verification**

Run: `make check-ai` with `CONTRACT` omitted after archive, then inspect `.ai/work-items/archive/2026/wi-wr-014.archive-manifest.json` and the matching archived Contract, Summary, and Outcome files.

Expected: archive digests and Contract/Summary/Outcome identity are consistent, with no duplicate archive sequence.

- [ ] **Step 4: Commit the complete local lifecycle**

Run: `git add .ai docs/superpowers src/features/weekly_radar/domain tests/system_health_test.rs tests/weekly_radar_system_health.rs && git commit -m "feat: integrate weekly radar system health"`

Expected: a local commit is created on `codex/wi-wr-014-system-health`; do not push, open a PR, merge, or close the Work Item.
