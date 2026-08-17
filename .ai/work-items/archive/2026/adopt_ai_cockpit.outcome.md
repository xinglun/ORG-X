# Task Outcome: adopt_ai_cockpit

Status: `completed_with_warnings`
Human Status: `yellow`

## Outcome Summary
Task adopt_ai_cockpit generated an evidence-derived outcome with status completed_with_warnings.

## Task Overview
Governed Work Item: adopt_ai_cockpit

## Delivered Changes
- .ai/README.md
- .ai/calibration/profiles.yaml
- .ai/cockpit/README.ja.md
- .ai/cockpit/README.md
- .ai/cockpit/adoption-runtime-verification.json
- .ai/cockpit/adoption.ja.md
- .ai/cockpit/adoption.md
- .ai/cockpit/checks.yaml
- .ai/cockpit/current_status.md
- .ai/cockpit/derived_artifacts.json
- .ai/cockpit/system_invariants.json
- .ai/cockpit/version.json
- .ai/cockpit/work-items/index.json
- .ai/cockpit/work-items/wi-06-status-interface.status.json
- .ai/decisions/.gitkeep
- .ai/evidence/test-weakening/documentation-p0-comprehension-ci-evidence.json
- .ai/evidence/test-weakening/documentation-p0-comprehension-evidence.json
- .ai/evidence/test-weakening/foreign-duplicate-start-v1.json
- .ai/evidence/test-weakening/remove-cancelled-provider-validation.json
- .ai/evidence/test-weakening/v057-public-projection-role-retirement.json
- .ai/glossary.md
- .ai/guards/agent_risk_policy.yaml
- .ai/guards/ai_review_policy.yaml
- .ai/guards/backtrack_policy.yaml
- .ai/guards/changed_critical_coverage_policy.json
- .ai/guards/cockpit_status_policy.yaml
- .ai/guards/coverage_policy.yaml
- .ai/guards/file_boundary.yaml
- .ai/guards/file_ownership.yaml
- .ai/guards/governance_complexity_policy.yaml
- .ai/guards/operation_impact_policy.yaml
- .ai/guards/preflight_review_policy.yaml
- .ai/guards/reference_impact_policy.yaml
- .ai/guards/scenario_coverage_policy.yaml
- .ai/guards/scope_policy.yaml
- .ai/guards/summary_policy.yaml
- .ai/guards/test_weakening_policy.yaml
- .ai/install/managed-regions.json
- .ai/install/manifest.json
- .ai/install/release-identity.json
- .ai/install/rollback-baseline.json
- .ai/install/version.json
- .ai/policies/complexity_trend.yaml
- .ai/policies/raw-request-exemptions.yaml
- .ai/policies/requested-operation.yaml
- .ai/policies/verification_impact.yaml
- .ai/project/capabilities.json
- .ai/project/success_criteria.json
- .ai/quality/gates.yaml
- .ai/quality/governance-routing.yaml
- .ai/schemas/canonical_evidence.schema.json
- .ai/schemas/cross-wi-integration-report.schema.json
- .ai/schemas/evidence-binding.schema.json
- .ai/schemas/external_handoff.schema.json
- .ai/schemas/governance-cost-report.schema.json
- .ai/schemas/operation-impact.schema.json
- .ai/schemas/parallel-verification-plan.schema.json
- .ai/schemas/performance-diagnosis-report.schema.json
- .ai/schemas/reference_impact.schema.json
- .ai/schemas/task_outcome.schema.json
- .ai/schemas/test_weakening.schema.json
- .ai/schemas/unknown_assessment.schema.json
- .ai/schemas/work-item-intelligence-snapshot.schema.json
- .ai/schemas/work-item-status-interface.schema.json
- .ai/trust/schema/approval.schema.json
- .ai/trust/schema/baseline_evidence.schema.json
- .ai/trust/schema/human_decision_evidence.schema.json
- .ai/trust/schema/human_decision_request.schema.json
- .ai/trust/schema/repository_capabilities.schema.json
- .ai/trust/schema/success_criteria.schema.json
- .ai/work-items/_templates/work_item_contract.example.json
- .ai/work-items/_templates/work_item_summary.example.json
- .ai/work-items/active/adopt_ai_cockpit.contract.json
- .ai/work-items/active/adopt_ai_cockpit.summary.json
- .ai/work-items/conflict-successor-receipts/wi-08-content-bound-reuse-collision-source.json
- .ai/work-items/conflict-successor-receipts/wi-10-environment-bound-collision-source.json
- .ai/work-items/conflict-successor-receipts/wi-11-governance-profile-effect-collision-source.json
- .ai/work-items/conflict-successor-receipts/wi-12-performance-diagnosis-collision-source.json
- .ai/work-items/recovery-receipts/jdk-lane-runtime-validation-620-current-main-31298815487-93208171431.json
- .ai/work-items/recovery-receipts/jdk-lane-runtime-validation-620-current-main.json
- .ai/work-items/recovery-receipts/post-publish-version-truth-v0549-current-main-31328153138-93281833266.json
- .ai/work-items/recovery-receipts/post-publish-version-truth-v0549-current-main.json
- .ai/work-items/recovery-receipts/release-freeze-published-projection-repair.json
- .ai/work-items/recovery-receipts/wi-04-hosted-installation-recovery.json
- .ai/work-items/recovery-receipts/wi-09-external-identity-recovery.json
- .ai/work-items/recovery-receipts/wi-16-outcome-human-handoff-31951774632-95176288562.json
- .ai/work-items/recovery-receipts/wi-16-outcome-human-handoff.json
- .ai/work-items/recovery-receipts/wi-17-stale-code-doc-cleanup.json
- .ai/work-items/recovery-receipts/wi-21-outcome-resolution-projection.json
- .ai/work-items/recovery-receipts/wi10-stacked-pr-chain-20260730.json
- .ai/work-items/runtime/.gitignore
- .ai/work-items/starts/adopt_ai_cockpit.json
- .cursor/rules/ai-cockpit.mdc
- .gitignore
- AGENTS.md
- CLAUDE.md
- GEMINI.md
- Makefile
- Makefile.ai
- Makefile.ai.stack
- scripts/ai_acceptance_policy.py
- scripts/ai_adoption_evidence.py
- scripts/ai_archive_work_item.py
- scripts/ai_baseline_evidence.py
- scripts/ai_calibrate.py
- scripts/ai_calibration_corrective.py
- scripts/ai_calibration_inventory.py
- scripts/ai_calibration_profiles.py
- scripts/ai_check_adoption_ready.py
- scripts/ai_check_agent_risk.py
- scripts/ai_check_backtrack.py
- scripts/ai_check_budget_impact.py
- scripts/ai_check_coverage_guard.py
- scripts/ai_check_diff_ownership.py
- scripts/ai_check_guard_calibration.py
- scripts/ai_check_guards.py
- scripts/ai_check_guidelines.py
- scripts/ai_check_pr.py
- scripts/ai_check_reference_impact.py
- scripts/ai_check_review_policy.py
- scripts/ai_check_scenario_coverage.py
- scripts/ai_check_scope.py
- scripts/ai_check_serial_order.py
- scripts/ai_check_status.py
- scripts/ai_check_status_consistency.py
- scripts/ai_check_summary.py
- scripts/ai_check_task_outcome.py
- scripts/ai_check_test_weakening.py
- scripts/ai_check_work_item.py
- scripts/ai_checkpoint.py
- scripts/ai_classify_operation_impact.py
- scripts/ai_close_work_item.py
- scripts/ai_common.py
- scripts/ai_critical_domain_guards.py
- scripts/ai_decision_protocol.py
- scripts/ai_detached_uninstaller.py
- scripts/ai_disable_enable.py
- scripts/ai_doctor.py
- scripts/ai_evidence_dependencies.py
- scripts/ai_external_handoff.py
- scripts/ai_external_identity.py
- scripts/ai_finish.py
- scripts/ai_generate_human_report.py
- scripts/ai_generate_status.py
- scripts/ai_generate_task_outcome.py
- scripts/ai_governance_compression.py
- scripts/ai_impact_classifier.py
- scripts/ai_input_trust.py
- scripts/ai_install_facts.py
- scripts/ai_install_status.py
- scripts/ai_installer_bootstrap.py
- scripts/ai_installer_catalog.json
- scripts/ai_installer_detection.py
- scripts/ai_installer_evidence.py
- scripts/ai_installer_managed_regions.py
- scripts/ai_installer_ownership.py
- scripts/ai_installer_repository.py
- scripts/ai_installer_transaction.py
- scripts/ai_installer_upgrade.py
- scripts/ai_intent_policy.py
- scripts/ai_lifecycle_facts.py
- scripts/ai_lifecycle_truth.py
- scripts/ai_observability.py
- scripts/ai_onboard.py
- scripts/ai_ownership.py
- scripts/ai_post_archive_recovery.py
- scripts/ai_preflight_review.py
- scripts/ai_project_doctor.py
- scripts/ai_project_profile.py
- scripts/ai_projection_lease.py
- scripts/ai_readiness_policy.py
- scripts/ai_render_task_outcome.py
- scripts/ai_render_task_outcome_multilingual.py
- scripts/ai_required_evidence.py
- scripts/ai_review_readiness_policy.py
- scripts/ai_risk_policy.py
- scripts/ai_rollback.py
- scripts/ai_scenario_policy.py
- scripts/ai_start.py
- scripts/ai_start_receipt.py
- scripts/ai_task_event_log.py
- scripts/ai_trust_guards.py
- scripts/ai_trust_schema.py
- scripts/ai_uninstall_facts.py
- scripts/ai_uninstall_proposal.py
- scripts/ai_upgrade_apply.py
- scripts/ai_upgrade_conflict_report.py
- scripts/ai_upgrade_proposal.py
- scripts/ai_validate_java_runtime.py
- scripts/ai_verification_policy.py
- scripts/ai_work_item_intelligence.py
- scripts/ai_work_item_status.py
- scripts/bootstrap_repository.py
- scripts/bootstrap_wizard.py
- scripts/bootstrap_write_boundary.py
- scripts/check_changed_critical_coverage.py
- scripts/check_critical_coverage.py
- scripts/determine_governance_profile.py
- .ai/work-items/active/adopt_ai_cockpit.outcome.json
- .ai/work-items/active/adopt_ai_cockpit.outcome.md
- .ai/cockpit/task_report.json
- .ai/cockpit/task_report.md

## Findings
None

## Risks
None

## Warnings
- Project-specific quality commands are not configured by adoption.

## Limitations
- Unresolved evidence is explicitly limited

## Non-Risk Explanations
- {"evidence": [], "reason": "The Summary records this item as an unresolved gap rather than a verified result.", "sourceWarning": "Project-specific quality commands are not configured by adoption."}

## Forbidden Claims
- Do not claim an unresolved warning was verified or resolved.

## Interventions
None

## Forced Stops
None

## Resolutions
None

## Recurrence Prevention
None

## Avoided Impact
None

## Residual Risks
- project_quality

## Human Decisions
None

## Evidence
- Contract
- Summary

## Human Handoff
Locale: `zh-CN`

### What was completed
- Changed .ai/README.md: from .ai/README.md
- Changed .ai/calibration/profiles.yaml: from .ai/calibration/profiles.yaml
- Changed .ai/cockpit/README.ja.md: from .ai/cockpit/README.ja.md
- Changed .ai/cockpit/README.md: from .ai/cockpit/README.md
- Changed .ai/cockpit/adoption-runtime-verification.json: write adopter Runtime Verification evidence
- Changed .ai/cockpit/adoption.ja.md: from .ai/cockpit/adoption.ja.md
- Changed .ai/cockpit/adoption.md: from .ai/cockpit/adoption.md
- Changed .ai/cockpit/checks.yaml: from .ai/cockpit/checks.yaml
- Changed .ai/cockpit/current_status.md: generate adoption Work Item status
- Changed .ai/cockpit/derived_artifacts.json: from .ai/cockpit/derived_artifacts.json
- Changed .ai/cockpit/system_invariants.json: from .ai/cockpit/system_invariants.json
- Changed .ai/cockpit/version.json: from .ai/cockpit/version.json
- Changed .ai/cockpit/work-items/index.json: from .ai/cockpit/work-items/index.json
- Changed .ai/cockpit/work-items/wi-06-status-interface.status.json: from .ai/cockpit/work-items/wi-06-status-interface.status.json
- Changed .ai/decisions/.gitkeep: from .ai/decisions/.gitkeep
- Changed .ai/evidence/test-weakening/documentation-p0-comprehension-ci-evidence.json: from .ai/evidence/test-weakening/documentation-p0-comprehension-ci-evidence.json
- Changed .ai/evidence/test-weakening/documentation-p0-comprehension-evidence.json: from .ai/evidence/test-weakening/documentation-p0-comprehension-evidence.json
- Changed .ai/evidence/test-weakening/foreign-duplicate-start-v1.json: from .ai/evidence/test-weakening/foreign-duplicate-start-v1.json
- Changed .ai/evidence/test-weakening/remove-cancelled-provider-validation.json: from .ai/evidence/test-weakening/remove-cancelled-provider-validation.json
- Changed .ai/evidence/test-weakening/v057-public-projection-role-retirement.json: from .ai/evidence/test-weakening/v057-public-projection-role-retirement.json
- Changed .ai/glossary.md: install project glossary template
- Changed .ai/guards/agent_risk_policy.yaml: from .ai/guards/agent_risk_policy.yaml
- Changed .ai/guards/ai_review_policy.yaml: from .ai/guards/ai_review_policy.yaml
- Changed .ai/guards/backtrack_policy.yaml: from .ai/guards/backtrack_policy.yaml
- Changed .ai/guards/changed_critical_coverage_policy.json: from .ai/guards/changed_critical_coverage_policy.json
- Changed .ai/guards/cockpit_status_policy.yaml: from .ai/guards/cockpit_status_policy.yaml
- Changed .ai/guards/coverage_policy.yaml: from .ai/guards/coverage_policy.yaml
- Changed .ai/guards/file_boundary.yaml: from .ai/guards/file_boundary.yaml
- Changed .ai/guards/file_ownership.yaml: from .ai/guards/file_ownership.yaml
- Changed .ai/guards/governance_complexity_policy.yaml: from .ai/guards/governance_complexity_policy.yaml
- Changed .ai/guards/operation_impact_policy.yaml: from .ai/guards/operation_impact_policy.yaml
- Changed .ai/guards/preflight_review_policy.yaml: from .ai/guards/preflight_review_policy.yaml
- Changed .ai/guards/reference_impact_policy.yaml: from .ai/guards/reference_impact_policy.yaml
- Changed .ai/guards/scenario_coverage_policy.yaml: from .ai/guards/scenario_coverage_policy.yaml
- Changed .ai/guards/scope_policy.yaml: from .ai/guards/scope_policy.yaml
- Changed .ai/guards/summary_policy.yaml: from .ai/guards/summary_policy.yaml
- Changed .ai/guards/test_weakening_policy.yaml: from .ai/guards/test_weakening_policy.yaml
- Changed .ai/install/managed-regions.json: write installed lifecycle fact
- Changed .ai/install/manifest.json: write installed lifecycle fact
- Changed .ai/install/release-identity.json: write installed lifecycle fact
- Changed .ai/install/rollback-baseline.json: write installed lifecycle fact
- Changed .ai/install/version.json: write installed lifecycle fact
- Changed .ai/policies/complexity_trend.yaml: from .ai/policies/complexity_trend.yaml
- Changed .ai/policies/raw-request-exemptions.yaml: from .ai/policies/raw-request-exemptions.yaml
- Changed .ai/policies/requested-operation.yaml: from .ai/policies/requested-operation.yaml
- Changed .ai/policies/verification_impact.yaml: from .ai/policies/verification_impact.yaml
- Changed .ai/project/capabilities.json: from .ai/project/capabilities.json
- Changed .ai/project/success_criteria.json: from .ai/project/success_criteria.json
- Changed .ai/quality/gates.yaml: from .ai/quality/gates.yaml
- Changed .ai/quality/governance-routing.yaml: from .ai/quality/governance-routing.yaml
- Changed .ai/schemas/canonical_evidence.schema.json: from .ai/schemas/canonical_evidence.schema.json
- Changed .ai/schemas/cross-wi-integration-report.schema.json: from .ai/schemas/cross-wi-integration-report.schema.json
- Changed .ai/schemas/evidence-binding.schema.json: from .ai/schemas/evidence-binding.schema.json
- Changed .ai/schemas/external_handoff.schema.json: from .ai/schemas/external_handoff.schema.json
- Changed .ai/schemas/governance-cost-report.schema.json: from .ai/schemas/governance-cost-report.schema.json
- Changed .ai/schemas/operation-impact.schema.json: from .ai/schemas/operation-impact.schema.json
- Changed .ai/schemas/parallel-verification-plan.schema.json: from .ai/schemas/parallel-verification-plan.schema.json
- Changed .ai/schemas/performance-diagnosis-report.schema.json: from .ai/schemas/performance-diagnosis-report.schema.json
- Changed .ai/schemas/reference_impact.schema.json: from .ai/schemas/reference_impact.schema.json
- Changed .ai/schemas/task_outcome.schema.json: from .ai/schemas/task_outcome.schema.json
- Changed .ai/schemas/test_weakening.schema.json: from .ai/schemas/test_weakening.schema.json
- Changed .ai/schemas/unknown_assessment.schema.json: from .ai/schemas/unknown_assessment.schema.json
- Changed .ai/schemas/work-item-intelligence-snapshot.schema.json: from .ai/schemas/work-item-intelligence-snapshot.schema.json
- Changed .ai/schemas/work-item-status-interface.schema.json: from .ai/schemas/work-item-status-interface.schema.json
- Changed .ai/trust/schema/approval.schema.json: from .ai/trust/schema/approval.schema.json
- Changed .ai/trust/schema/baseline_evidence.schema.json: from .ai/trust/schema/baseline_evidence.schema.json
- Changed .ai/trust/schema/human_decision_evidence.schema.json: from .ai/trust/schema/human_decision_evidence.schema.json
- Changed .ai/trust/schema/human_decision_request.schema.json: from .ai/trust/schema/human_decision_request.schema.json
- Changed .ai/trust/schema/repository_capabilities.schema.json: from .ai/trust/schema/repository_capabilities.schema.json
- Changed .ai/trust/schema/success_criteria.schema.json: from .ai/trust/schema/success_criteria.schema.json
- Changed .ai/work-items/_templates/work_item_contract.example.json: from .ai/work-items/_templates/work_item_contract.example.json
- Changed .ai/work-items/_templates/work_item_summary.example.json: from .ai/work-items/_templates/work_item_summary.example.json
- Changed .ai/work-items/active/adopt_ai_cockpit.contract.json: create adoption Contract
- Changed .ai/work-items/active/adopt_ai_cockpit.summary.json: create adoption Summary
- Changed .ai/work-items/conflict-successor-receipts/wi-08-content-bound-reuse-collision-source.json: from .ai/work-items/conflict-successor-receipts/wi-08-content-bound-reuse-collision-source.json
- Changed .ai/work-items/conflict-successor-receipts/wi-10-environment-bound-collision-source.json: from .ai/work-items/conflict-successor-receipts/wi-10-environment-bound-collision-source.json
- Changed .ai/work-items/conflict-successor-receipts/wi-11-governance-profile-effect-collision-source.json: from .ai/work-items/conflict-successor-receipts/wi-11-governance-profile-effect-collision-source.json
- Changed .ai/work-items/conflict-successor-receipts/wi-12-performance-diagnosis-collision-source.json: from .ai/work-items/conflict-successor-receipts/wi-12-performance-diagnosis-collision-source.json
- Changed .ai/work-items/recovery-receipts/jdk-lane-runtime-validation-620-current-main-31298815487-93208171431.json: from .ai/work-items/recovery-receipts/jdk-lane-runtime-validation-620-current-main-31298815487-93208171431.json
- Changed .ai/work-items/recovery-receipts/jdk-lane-runtime-validation-620-current-main.json: from .ai/work-items/recovery-receipts/jdk-lane-runtime-validation-620-current-main.json
- Changed .ai/work-items/recovery-receipts/post-publish-version-truth-v0549-current-main-31328153138-93281833266.json: from .ai/work-items/recovery-receipts/post-publish-version-truth-v0549-current-main-31328153138-93281833266.json
- Changed .ai/work-items/recovery-receipts/post-publish-version-truth-v0549-current-main.json: from .ai/work-items/recovery-receipts/post-publish-version-truth-v0549-current-main.json
- Changed .ai/work-items/recovery-receipts/release-freeze-published-projection-repair.json: from .ai/work-items/recovery-receipts/release-freeze-published-projection-repair.json
- Changed .ai/work-items/recovery-receipts/wi-04-hosted-installation-recovery.json: from .ai/work-items/recovery-receipts/wi-04-hosted-installation-recovery.json
- Changed .ai/work-items/recovery-receipts/wi-09-external-identity-recovery.json: from .ai/work-items/recovery-receipts/wi-09-external-identity-recovery.json
- Changed .ai/work-items/recovery-receipts/wi-16-outcome-human-handoff-31951774632-95176288562.json: from .ai/work-items/recovery-receipts/wi-16-outcome-human-handoff-31951774632-95176288562.json
- Changed .ai/work-items/recovery-receipts/wi-16-outcome-human-handoff.json: from .ai/work-items/recovery-receipts/wi-16-outcome-human-handoff.json
- Changed .ai/work-items/recovery-receipts/wi-17-stale-code-doc-cleanup.json: from .ai/work-items/recovery-receipts/wi-17-stale-code-doc-cleanup.json
- Changed .ai/work-items/recovery-receipts/wi-21-outcome-resolution-projection.json: from .ai/work-items/recovery-receipts/wi-21-outcome-resolution-projection.json
- Changed .ai/work-items/recovery-receipts/wi10-stacked-pr-chain-20260730.json: from .ai/work-items/recovery-receipts/wi10-stacked-pr-chain-20260730.json
- Changed .ai/work-items/runtime/.gitignore: from .ai/work-items/runtime/.gitignore
- Changed .ai/work-items/starts/adopt_ai_cockpit.json: create adoption Start Receipt
- Changed .cursor/rules/ai-cockpit.mdc: from .cursor/rules/ai-cockpit.mdc
- Changed .gitignore: add missing AI Cockpit local-state ignore rules
- Changed AGENTS.md: install managed AI Cockpit section
- Changed CLAUDE.md: install managed AI Cockpit section
- Changed GEMINI.md: install managed AI Cockpit section
- Changed Makefile: include Makefile.ai
- Changed Makefile.ai: from templates/make/Makefile.ai
- Changed Makefile.ai.stack: from templates/stacks/rust.mk
- Changed scripts/ai_acceptance_policy.py: from scripts/ai_acceptance_policy.py
- Changed scripts/ai_adoption_evidence.py: from scripts/ai_adoption_evidence.py
- Changed scripts/ai_archive_work_item.py: from scripts/ai_archive_work_item.py
- Changed scripts/ai_baseline_evidence.py: from scripts/ai_baseline_evidence.py
- Changed scripts/ai_calibrate.py: from scripts/ai_calibrate.py
- Changed scripts/ai_calibration_corrective.py: from scripts/ai_calibration_corrective.py
- Changed scripts/ai_calibration_inventory.py: from scripts/ai_calibration_inventory.py
- Changed scripts/ai_calibration_profiles.py: from scripts/ai_calibration_profiles.py
- Changed scripts/ai_check_adoption_ready.py: from scripts/ai_check_adoption_ready.py
- Changed scripts/ai_check_agent_risk.py: from scripts/ai_check_agent_risk.py
- Changed scripts/ai_check_backtrack.py: from scripts/ai_check_backtrack.py
- Changed scripts/ai_check_budget_impact.py: from scripts/ai_check_budget_impact.py
- Changed scripts/ai_check_coverage_guard.py: from scripts/ai_check_coverage_guard.py
- Changed scripts/ai_check_diff_ownership.py: from scripts/ai_check_diff_ownership.py
- Changed scripts/ai_check_guard_calibration.py: from scripts/ai_check_guard_calibration.py
- Changed scripts/ai_check_guards.py: from scripts/ai_check_guards.py
- Changed scripts/ai_check_guidelines.py: from scripts/ai_check_guidelines.py
- Changed scripts/ai_check_pr.py: from scripts/ai_check_pr.py
- Changed scripts/ai_check_reference_impact.py: from scripts/ai_check_reference_impact.py
- Changed scripts/ai_check_review_policy.py: from scripts/ai_check_review_policy.py
- Changed scripts/ai_check_scenario_coverage.py: from scripts/ai_check_scenario_coverage.py
- Changed scripts/ai_check_scope.py: from scripts/ai_check_scope.py
- Changed scripts/ai_check_serial_order.py: from scripts/ai_check_serial_order.py
- Changed scripts/ai_check_status.py: from scripts/ai_check_status.py
- Changed scripts/ai_check_status_consistency.py: from scripts/ai_check_status_consistency.py
- Changed scripts/ai_check_summary.py: from scripts/ai_check_summary.py
- Changed scripts/ai_check_task_outcome.py: from scripts/ai_check_task_outcome.py
- Changed scripts/ai_check_test_weakening.py: from scripts/ai_check_test_weakening.py
- Changed scripts/ai_check_work_item.py: from scripts/ai_check_work_item.py
- Changed scripts/ai_checkpoint.py: from scripts/ai_checkpoint.py
- Changed scripts/ai_classify_operation_impact.py: from scripts/ai_classify_operation_impact.py
- Changed scripts/ai_close_work_item.py: from scripts/ai_close_work_item.py
- Changed scripts/ai_common.py: from scripts/ai_common.py
- Changed scripts/ai_critical_domain_guards.py: from scripts/ai_critical_domain_guards.py
- Changed scripts/ai_decision_protocol.py: from scripts/ai_decision_protocol.py
- Changed scripts/ai_detached_uninstaller.py: from scripts/ai_detached_uninstaller.py
- Changed scripts/ai_disable_enable.py: from scripts/ai_disable_enable.py
- Changed scripts/ai_doctor.py: from scripts/ai_doctor.py
- Changed scripts/ai_evidence_dependencies.py: from scripts/ai_evidence_dependencies.py
- Changed scripts/ai_external_handoff.py: from scripts/ai_external_handoff.py
- Changed scripts/ai_external_identity.py: from scripts/ai_external_identity.py
- Changed scripts/ai_finish.py: from scripts/ai_finish.py
- Changed scripts/ai_generate_human_report.py: from scripts/ai_generate_human_report.py
- Changed scripts/ai_generate_status.py: from scripts/ai_generate_status.py
- Changed scripts/ai_generate_task_outcome.py: from scripts/ai_generate_task_outcome.py
- Changed scripts/ai_governance_compression.py: from scripts/ai_governance_compression.py
- Changed scripts/ai_impact_classifier.py: from scripts/ai_impact_classifier.py
- Changed scripts/ai_input_trust.py: from scripts/ai_input_trust.py
- Changed scripts/ai_install_facts.py: from scripts/ai_install_facts.py
- Changed scripts/ai_install_status.py: from scripts/ai_install_status.py
- Changed scripts/ai_installer_bootstrap.py: from scripts/ai_installer_bootstrap.py
- Changed scripts/ai_installer_catalog.json: from scripts/ai_installer_catalog.json
- Changed scripts/ai_installer_detection.py: from scripts/ai_installer_detection.py
- Changed scripts/ai_installer_evidence.py: from scripts/ai_installer_evidence.py
- Changed scripts/ai_installer_managed_regions.py: from scripts/ai_installer_managed_regions.py
- Changed scripts/ai_installer_ownership.py: from scripts/ai_installer_ownership.py
- Changed scripts/ai_installer_repository.py: from scripts/ai_installer_repository.py
- Changed scripts/ai_installer_transaction.py: from scripts/ai_installer_transaction.py
- Changed scripts/ai_installer_upgrade.py: from scripts/ai_installer_upgrade.py
- Changed scripts/ai_intent_policy.py: from scripts/ai_intent_policy.py
- Changed scripts/ai_lifecycle_facts.py: from scripts/ai_lifecycle_facts.py
- Changed scripts/ai_lifecycle_truth.py: from scripts/ai_lifecycle_truth.py
- Changed scripts/ai_observability.py: from scripts/ai_observability.py
- Changed scripts/ai_onboard.py: from scripts/ai_onboard.py
- Changed scripts/ai_ownership.py: from scripts/ai_ownership.py
- Changed scripts/ai_post_archive_recovery.py: from scripts/ai_post_archive_recovery.py
- Changed scripts/ai_preflight_review.py: from scripts/ai_preflight_review.py
- Changed scripts/ai_project_doctor.py: from scripts/ai_project_doctor.py
- Changed scripts/ai_project_profile.py: from scripts/ai_project_profile.py
- Changed scripts/ai_projection_lease.py: from scripts/ai_projection_lease.py
- Changed scripts/ai_readiness_policy.py: from scripts/ai_readiness_policy.py
- Changed scripts/ai_render_task_outcome.py: from scripts/ai_render_task_outcome.py
- Changed scripts/ai_render_task_outcome_multilingual.py: from scripts/ai_render_task_outcome_multilingual.py
- Changed scripts/ai_required_evidence.py: from scripts/ai_required_evidence.py
- Changed scripts/ai_review_readiness_policy.py: from scripts/ai_review_readiness_policy.py
- Changed scripts/ai_risk_policy.py: from scripts/ai_risk_policy.py
- Changed scripts/ai_rollback.py: from scripts/ai_rollback.py
- Changed scripts/ai_scenario_policy.py: from scripts/ai_scenario_policy.py
- Changed scripts/ai_start.py: from scripts/ai_start.py
- Changed scripts/ai_start_receipt.py: from scripts/ai_start_receipt.py
- Changed scripts/ai_task_event_log.py: from scripts/ai_task_event_log.py
- Changed scripts/ai_trust_guards.py: from scripts/ai_trust_guards.py
- Changed scripts/ai_trust_schema.py: from scripts/ai_trust_schema.py
- Changed scripts/ai_uninstall_facts.py: from scripts/ai_uninstall_facts.py
- Changed scripts/ai_uninstall_proposal.py: from scripts/ai_uninstall_proposal.py
- Changed scripts/ai_upgrade_apply.py: from scripts/ai_upgrade_apply.py
- Changed scripts/ai_upgrade_conflict_report.py: from scripts/ai_upgrade_conflict_report.py
- Changed scripts/ai_upgrade_proposal.py: from scripts/ai_upgrade_proposal.py
- Changed scripts/ai_validate_java_runtime.py: from scripts/ai_validate_java_runtime.py
- Changed scripts/ai_verification_policy.py: from scripts/ai_verification_policy.py
- Changed scripts/ai_work_item_intelligence.py: from scripts/ai_work_item_intelligence.py
- Changed scripts/ai_work_item_status.py: from scripts/ai_work_item_status.py
- Changed scripts/bootstrap_repository.py: from scripts/bootstrap_repository.py
- Changed scripts/bootstrap_wizard.py: from scripts/bootstrap_wizard.py
- Changed scripts/bootstrap_write_boundary.py: from scripts/bootstrap_write_boundary.py
- Changed scripts/check_changed_critical_coverage.py: from scripts/check_changed_critical_coverage.py
- Changed scripts/check_critical_coverage.py: from scripts/check_critical_coverage.py
- Changed scripts/determine_governance_profile.py: from scripts/determine_governance_profile.py
- Changed .ai/work-items/active/adopt_ai_cockpit.outcome.json: Mandatory Task Outcome evidence generated by ai-finish.
- Changed .ai/work-items/active/adopt_ai_cockpit.outcome.md: Mandatory Task Outcome evidence generated by ai-finish.
- Changed .ai/cockpit/task_report.json: Generated machine-readable Human Benefit Review Report.
- Changed .ai/cockpit/task_report.md: Generated human-readable Human Benefit Review Report.

### What passed
- aiWorkItem: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_work_item.py .ai/work-items/active/adopt_ai_cockpit.contract.json work item contract check passed: .ai/work-items/active/adopt_ai_cockpit.contract.json
- aiScope: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_scope.py .ai/work-items/active/adopt_ai_cockpit.contract.json scope guard passed: 198 changed path(s) covered
- aiStatus: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_generate_status.py .ai/work-items/active/adopt_ai_cockpit.contract.json --summary .ai/work-items/active/adopt_ai_cockpit.summary.json cockpit status generated: <PROJECT_ROOT>/.ai/cockpit/current_status.md
- aiStatusCheck: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_status.py .ai/cockpit/current_status.md --contract .ai/work-items/active/adopt_ai_cockpit.contract.json --summary .ai/work-items/active/adopt_ai_cockpit.summary.json cockpit status check passed: .ai/cockpit/current_status.md
- aiStatusConsistency: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_status_consistency.py ai status consistency check passed
- aiAgentRisk: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_agent_risk.py --contract .ai/work-items/active/adopt_ai_cockpit.contract.json --summary .ai/work-items/active/adopt_ai_cockpit.summary.json agent risk check passed report: target/ai_agent_risk_report.json
- aiSummary: PYTHONDONTWRITEBYTECODE=1 <LOCAL_PATH> scripts/ai_check_summary.py .ai/work-items/active/adopt_ai_cockpit.summary.json --contract .ai/work-items/active/adopt_ai_cockpit.contract.json ai summary check passed: .ai/work-items/active/adopt_ai_cockpit.summary.json

### What was retained
- Retained limitation: Project-specific quality commands are not configured by adoption.

### Risks
- project_quality: Configure and require project quality checks after adoption.

### Red reasons
None

### Human questions
- problemCount: 1
- blockedProblems: None
- resolvedProblems: None
- resolutionApproach: None
- avoidedRisks: None
- remainingRisks: Configure and require project quality checks after adoption.
- agentUnknowns: None
- humanConfirmations: None
- recurrenceLikelihood: unknown: no direct recurrence probability evidence was recorded.
- nextTime: Bind conversation locale and preserve evidence details before the next Work Item starts.
