# Task Outcome: wi-weekly-radar-structural-evidence-gate

Status: `completed`
Human Status: `green`

## Outcome Summary
Task wi-weekly-radar-structural-evidence-gate generated an evidence-derived outcome with status completed.

## Task Overview
Governed Work Item: wi-weekly-radar-structural-evidence-gate

## Delivered Changes
- .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json
- .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.summary.json
- .ai/work-items/starts/wi-weekly-radar-structural-evidence-gate.json
- .ai/cockpit/current_status.md
- .ai/cockpit/task_report.json
- .ai/cockpit/task_report.md
- .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.outcome.json
- .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.outcome.md
- docs/superpowers/specs/2026-08-25-weekly-radar-structural-evidence-gate-design.md
- src/features/weekly_radar/runtime/evidence.rs
- src/features/weekly_radar/runtime/model.rs
- src/features/weekly_radar/runtime/report.rs
- src/features/weekly_radar/interface/semantic_message_splitter.rs
- src/main.rs
- tests/weekly_radar_evidence_quality.rs
- tests/weekly_radar_runtime.rs
- tests/weekly_radar_semantic_message_splitter.rs
- tests/semantic_message_splitter_test.rs
- docs/operations/WEEKLY_RADAR.md
- docs/superpowers/plans/2026-08-25-weekly-radar-structural-evidence-gate.md
- .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json
- .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.summary.json
- .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.outcome.json
- .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.outcome.md
- .ai/cockpit/task_report.json
- .ai/cockpit/task_report.md

## Findings
None

## Risks
None

## Warnings
None

## Limitations
None

## Non-Risk Explanations
None

## Forbidden Claims
None

## Interventions
None

## Forced Stops
- verification
- verification

## Resolutions
- aiGuidelines failed before the retry.
- aiSummary failed before the retry.

## Recurrence Prevention
None

## Avoided Impact
- If not detected, could have led to a stale completion claim.

## Residual Risks
- classification
- live-validation

## Human Decisions
- Keep fail-closed Ranking behavior.
- Separate source availability, leads, validated facts, and structural evidence.
- Prioritize truthful SEC stage/fact health.
- Execute the governed Work Item through CI and one authorized dry-run.

## Evidence
- Contract
- Summary
- approved design boundary
- operator-facing four-layer semantics
- verificationHistory[0] aiGuidelines failed
- verification[aiGuidelines] retry passed
- verificationHistory[1] aiSummary failed
- verification[aiSummary] retry passed

## Implementation Approach
Status: `complete`
Customer summary (verified): Keep the existing SourceObservation to EvidenceCandidate to ValidatedEvidence pipeline, then add a deterministic second classification that separates ordinary validated facts from structural evidence without changing Stage or Ranking gates.
Mechanism (verified): Classify only validated bounded passage text using fixed structural signals; aggregate SEC submissions/Company Facts stage reachability separately from normalized FactStatus::Known facts; render both dimensions read-only in localized reports.

Affected components
- Evidence classification: Validated document claims receive regular or structural evidence prefixes while preserving provenance. (verified)
- SEC health reporting: Stage availability and usable normalized facts are counted independently. (verified)
- Reader-facing report: Validated Facts and Structural Evidence are separate localized sections; old splitter aliases remain accepted. (verified)

Design decisions
- Prefer false negatives over promoting generic engineering prose.: A validated technical article is not evidence of enterprise production-system change without an explicit structural signal. (verified)
- Do not equate SEC stage reachability with usable SEC facts.: A reachable endpoint can return unavailable or empty normalized facts; readers need both counters. (verified)

### Technical details
- Compatibility: New ResearchMetrics fields default to zero during legacy RuntimeReportInput deserialization. (verified)
- Verification: Focused evidence/runtime/splitter suites and make quality pass; hosted CI and post-merge dry-run remain lifecycle steps. (verified)

### Evidence
- The implementation preserves the approved four-layer evidence boundary and does not promote page availability into structural evidence.: docs/superpowers/specs/2026-08-25-weekly-radar-structural-evidence-gate-design.md#approved design boundary (verified)
- The operator guide explains validated facts, structural evidence, SEC stage health, and usable SEC facts without claiming a live run.: docs/operations/WEEKLY_RADAR.md#operator-facing four-layer semantics (verified)

## Human Handoff
Locale: `zh-CN`

### What was completed
- Changed .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json: Binds the approved structural-evidence, SEC-health, scope, acceptance, and lifecycle authority.
- Changed .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.summary.json: Records implementation, verification, issue resolution, and residual-risk evidence.
- Changed .ai/work-items/starts/wi-weekly-radar-structural-evidence-gate.json: Immutable Start Receipt binds the dedicated Work Item branch to the recorded base.
- Changed .ai/cockpit/current_status.md: Generated AI Cockpit status projection for the active Work Item.
- Changed .ai/cockpit/task_report.json: Generated Human Benefit Report JSON during Finish.
- Changed .ai/cockpit/task_report.md: Generated localized Human Benefit Report Markdown during Finish.
- Changed .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.outcome.json: Mandatory Task Outcome evidence generated by ai-finish.
- Changed .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.outcome.md: Mandatory localized Task Outcome evidence generated by ai-finish.
- Changed docs/superpowers/specs/2026-08-25-weekly-radar-structural-evidence-gate-design.md: Defines the structural-evidence classification and SEC stage/fact semantics.
- Changed src/features/weekly_radar/runtime/evidence.rs: Added deterministic StructuralEvidence classification and stable normalized-fact kind prefixes.
- Changed src/features/weekly_radar/runtime/model.rs: Added backward-compatible structural and SEC stage/fact health metrics.
- Changed src/features/weekly_radar/runtime/report.rs: Separated validated facts from structural evidence and rendered localized SEC health semantics.
- Changed src/features/weekly_radar/interface/semantic_message_splitter.rs: Accepted new localized evidence headings while retaining legacy aliases.
- Changed src/main.rs: Bound structural classification and distinct SEC stage/fact counters during acquisition.
- Changed tests/weekly_radar_evidence_quality.rs: Added positive/negative classification, metrics, SEC health, and report regressions.
- Changed tests/weekly_radar_runtime.rs: Updated report semantics and retained runtime/archive regression coverage.
- Changed tests/weekly_radar_semantic_message_splitter.rs: Added new localized evidence heading splitting coverage.
- Changed tests/semantic_message_splitter_test.rs: Added structural evidence boundary coverage.
- Changed docs/operations/WEEKLY_RADAR.md: Documented four evidence layers, structural gate, and SEC stage-versus-fact semantics.
- Changed docs/superpowers/plans/2026-08-25-weekly-radar-structural-evidence-gate.md: Recorded completed TDD tasks and remaining lifecycle checks.
- Changed .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json: Active Work Item Contract is committed snapshot evidence.
- Changed .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.summary.json: Active AI Change Summary is committed snapshot evidence.
- Changed .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.outcome.json: Mandatory Task Outcome evidence generated by ai-finish.
- Changed .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.outcome.md: Mandatory Task Outcome evidence generated by ai-finish.
- Changed .ai/cockpit/task_report.json: Generated machine-readable Human Benefit Review Report.
- Changed .ai/cockpit/task_report.md: Generated human-readable Human Benefit Review Report.

### What passed
- aiWorkItem: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_work_item.py .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json work item contract check passed: .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json
- aiScope: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_scope.py .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json scope guard passed: 20 changed path(s) covered
- aiGuards: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_guards.py --contract .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json guard check completed: 0 warning(s) report: target/ai_guard_report.json
- aiCheckpoint: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_checkpoint.py --contract .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json --summary .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.summary.json --stage "before_finish" # AI Work Item Checkpoint - Stage: `before_finish` - Work Item: `wi-weekly-radar-structural-evidence-gate` - Contract Hash: `cc1f4c025e028882` - Mode: `code` - notCodable: `False` - Execution Decision: `continue` - Acceptance Count: `10`
- aiReviewPolicy: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_review_policy.py --summary .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.summary.json review policy matched 7 path(s) [review] .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json [review] .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.outcome.json [review] .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.outcome.md [review] .ai/work-items/starts/wi-weekly-radar-structur
- aiBacktrack: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_backtrack.py backtrack guard: no issues report: target/ai_backtrack_report.json
- aiCoverage: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_coverage_guard.py coverage guard: no issues report: target/ai_coverage_guard_report.json
- aiScenarioCoverage: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_scenario_coverage.py --contract .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json --summary .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.summary.json [warning] missing_scenario_coverage: - scenario coverage is missing for medium/high risk report: target/ai_scenario_coverage_report.json
- aiGuidelines: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_guidelines.py --contract .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json --summary .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.summary.json guidelines compliance check passed: 6 guideline(s) verified
- aiDiffOwnership: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_diff_ownership.py --contract .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json ## Diff Ownership Preview - active_owned: `20`, ambiguous: `0`, approval_required: `0`, archived_owned: `0`, out_of_scope: `0`, unowned: `0` - [active_owned] `.ai/cockpit/current_status.md` — covered by Contract scope - [active_owned] `.ai/cockpit/task_report.json` — exact generated Human Benefit Report pair validates against active Tas
- quality: {"finishQualityRoute": {"command": "make ai-cockpit-quality GOVERNANCE_PROFILE=strict", "policy": {"domains": ["docs", "project_code", "tests"], "level": "strict", "qualityRouting": {"reason": "explicit strict governance requires the complete quality graph", "requiredGroups": ["quality-full"], "target": "quality-full"}, "qualityTarget": "quality-full", "requiredGroups": ["quality-full"], "scope": "full", "stage": "task"}}} { "automaticProfile": "standard", "base": "caeb66cf732d35155b25b098a3b378
- aiStatus: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_generate_status.py .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json --summary .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.summary.json cockpit status generated: <PROJECT_ROOT>/.ai/cockpit/current_status.md
- aiStatusCheck: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_status.py .ai/cockpit/current_status.md --contract .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json --summary .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.summary.json cockpit status check passed: .ai/cockpit/current_status.md
- aiStatusConsistency: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_status_consistency.py ai status consistency check passed
- aiAgentRisk: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_agent_risk.py --contract .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json --summary .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.summary.json agent risk check passed report: target/ai_agent_risk_report.json
- aiSummary: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_summary.py .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.summary.json --contract .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json ai summary check passed: .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.summary.json

### What was retained
None

### Risks
- classification: The classifier is deterministic and intentionally conservative; real provider wording may remain a regular validated fact until the signal vocabulary is amended in a governed follow-up.
- live-validation: No post-merge dry-run has been executed for this Work Item yet; live SEC/source availability and the resulting structural count remain unverified until the authorized dispatch.

### Red reasons
None

### Human questions
- problemCount: 3
- blockedProblems: None
- resolvedProblems: aiGuidelines failed before the retry.; aiSummary failed before the retry.
- resolutionApproach: Re-ran aiGuidelines after the correction; the latest attempt passed.; Re-ran aiSummary after the correction; the latest attempt passed.
- avoidedRisks: If not detected, could have led to a stale completion claim.; If not detected, could have led to a stale completion claim.
- remainingRisks: Reader-facing headings changed from confirmed-information wording to validated-fact and structural-evidence wording; legacy splitter aliases remain accepted.; The classifier is deterministic and intentionally conservative; real provider wording may remain a regular validated fact until the signal vocabulary is amended in a governed follow-up.; No post-merge dry-run has been executed for this Work Item yet; live SEC/source availability and the resulting structural count remain unverified until the authorized dispatch.
- agentUnknowns: None
- humanConfirmations: Keep fail-closed Ranking behavior.; Separate source availability, leads, validated facts, and structural evidence.; Prioritize truthful SEC stage/fact health.; Execute the governed Work Item through CI and one authorized dry-run.
- recurrenceLikelihood: unknown: no direct recurrence probability evidence was recorded.
- nextTime: Bind conversation locale and preserve evidence details before the next Work Item starts.
