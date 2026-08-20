---
author: Ray
title: "Human-Agent Trust Layer"
description: Why AI Cockpit exists, what it governs, and how evidence, fail-closed control, trusted chains, supply-chain evidence, and human decisions work together.
keywords: [ai-cockpit, trust-layer, human-agent-trust, evidence, human-decision]
---

# Human-Agent Trust Layer

AI Cockpit is a Repository Governance Layer. It uses reviewable evidence to decide when a governed change may continue, when control must return to a human, and when the governed path must stop. It is not a SKILL, an Agent Runtime, or a Security Sandbox.

The three language versions of this document are complete equivalents. The English version is authoritative for wording; the Chinese and Japanese versions preserve the same structure, boundaries, implementation evidence, and limitations.

<!-- section-id: why -->
## Why

AI Cockpit exists because an agent can produce a plausible explanation without producing the evidence needed to trust a repository change. Human-Agent Trust is therefore calibrated, not absolute: the system exposes what is known, what is missing, who must decide, and how to recover.

The central rule is Evidence over Self-Declaration. AI Cockpit governs evidence; it does not replace evidence-producing tools. A chat assertion, an agent's confidence, or a self-declared approval is not an independent authorization.

The Trust Layer connects five responsibilities: repository governance, fail-closed control, trusted chains, delegated domain evidence, and human decisions. It makes the safe next action legible without claiming that a local repository layer proves every property of an organization, provider, model, or production environment.

<!-- section-id: what -->
## What

AI Cockpit governs a repository-local decision boundary. It binds a human request to a Work Item Contract, evaluates policy and scope, records verification, compresses the result for human review, and archives the evidence needed to reconstruct the decision.

The seven governance layers are:

1. **Execution boundary** — bind the requested operation, scope, authority, and allowed effect.
2. **Control return** — stop or ask for a human decision when evidence is missing, stale, contradictory, or high risk.
3. **Known-risk protection** — reject the deterministic injection, unsupported-claim, absurd, bypass, and unsafe-operation cases covered by repository gates.
4. **Complete trusted chain** — connect SHA-256, Git History, Digital Signature, Branch Protection, Hosted CI / External Audit Evidence, and Human Approval where each is actually produced.
5. **Software supply-chain evidence** — record SBOM, Provenance, release identity, checksums, scanning, and provider evidence without pretending delegated evidence is native proof.
6. **Human decision compression** — show state, trust signal, change, problems, stop reason, unknowns, decision, evidence basis, and next action without a score or confidence theater.
7. **Archive and recovery** — preserve Contract, Summary, events, decision records, release evidence, and Archive Manifest so a stopped or completed path can be reviewed and recovered.

The complete trusted chain is a composition, not a single AI Cockpit feature. SHA-256 binds bytes; Git History binds repository ancestry; Digital Signature binds a signer when an external signing system supplies it; Branch Protection binds hosted repository policy; Hosted CI / External Audit Evidence binds provider or auditor results; Human Approval records the responsible decision. Missing links remain missing.

SBOM and Provenance are different. An SBOM describes components and dependency relationships in a software artifact. Provenance describes how an artifact was produced, from which source, by which build process, and under which identity or environment. SBOM is Delegated Domain Evidence: AI Cockpit can record and govern it, but does not generate or independently validate the domain fact merely because a file exists.

AI Cockpit does not independently guarantee identity, runtime isolation, an immutable audit log, branch protection, digital signing, vulnerability absence, or enterprise compliance. AI Cockpit is not a Security Sandbox. Enterprise controls remain the responsibility of the adopter and the relevant providers or auditors.

<!-- section-id: how -->
## How

The governed path is:

```text
Human intent → Raw Request Binding → Work Item Contract → Preflight
→ Requested Operation / Capability Mapping → Change
→ Verification and external evidence → Human Decision and Recovery
→ Task Outcome / Status → Archive Manifest
```

Raw Request Binding preserves the request that established the work. Requested Operation makes target, action, environment, effect, and authority requirements explicit. Capability Mapping derives required capabilities from repository policy; a self-declared capability list cannot authorize an unmapped operation.

Preflight uses the enforced profile by default. A `ready` report may proceed. `not_ready`, `needs_human_confirmation`, `human_decision_recorded`, stale, contradictory, or failed evidence stops the governed path. A human decision resolves a workflow question; it does not turn an unverified check into a pass. Recovery means adding or correcting evidence and rerunning the affected checks.

Human Decision and Recovery must state what happened, why it matters, available options, the recommendation, evidence, and the resume condition. The resulting decision is archived with the Work Item; it is never used as a substitute for test, CI, security, release, identity, or enterprise-control evidence.

<!-- section-id: current-implementation -->
## Current Implementation

The current repository implements a local, deterministic Trust Layer. The following implemented details are part of the authority and must not be removed for conceptual cleanliness:

- **Unsupported Claim Regression Gate** (`make unsupported-claim-regression`) rejects unsupported completion, approval, execution, file, and release claims.
- **`delusion-test-gate`** (`make delusion-test-gate`) exercises the finite known-scenario regression vocabulary, including absurd, bypass, injection, and underspecified-work cases.
- **Guard Signal Envelope** carries `signalId`, `state`, `confidence`, `evidence`, `policyReference`, `humanDecisionAllowed`, and `safeAlternatives`, alongside legacy `name`, `value`, and `sources` fields. Deterministic confidence is evidence quality, never authority.
- **Preflight enforced profile** is configured in `.ai/guards/preflight_review_policy.yaml`; only a newly computed `ready` report proceeds through governed start and finish.
- **Raw Request Binding**, **Requested Operation**, and **Capability Mapping** are required Contract v2 boundaries for applicable code Work Items.
- **Human Decision and Recovery** persists structured requests and evidence, then requires Preflight and project checks to run again.
- **Archive Manifest** records SHA-256 digests of frozen Contract and Summary evidence in a non-self-referential archive record.

These are repository-local implementation facts. They do not prove universal semantic risk classification, general Japanese model fluency, provider identity, runtime isolation, or enterprise readiness. The WI-16 Japanese assessment remains bounded to deterministic Japanese governance paths; its general-fluency non-claim is intentional.

<!-- section-id: deterministic-coverage -->
## Deterministic Coverage

The gates cover a finite vocabulary of known, reviewable cases: missing or stale evidence, unsupported claims, invalid Work Item state, scope violations, raw-request and operation mismatches, selected prompt-injection indicators, unsafe critical-domain effects, and required human confirmation. They are fail-closed for the cases they recognize.

The [real absurd and injection assessment](reference/real-absurd-injection-cases.md) records twelve concrete negative cases and their current result. It distinguishes the five input-trust cases directly covered today from seven repository/lifecycle evidence gaps that require review; it does not infer malicious intent or turn an unbound gate into a protection claim.

They do not detect an agent's internal state, understand every language nuance, provide universal prompt-injection defense, or establish that an external control is configured. Capability Truth Matrix is the only source of current implementation status; a concept in this document cannot upgrade a capability from planned, template-only, adopter-installed, or externally required to implemented.

<!-- section-id: machine-readable-evidence -->
## Machine-Readable Evidence

The machine-readable evidence chain includes Contract v2, guard signals, Preflight reports, test and quality results, Task Outcome, Cockpit Status, human decision request/evidence, release evidence, and Archive Manifest. Each record has an owning lifecycle stage and should be referenced by path, command, commit, digest, or provider result.

Native Governance Evidence is produced by this repository's own governed commands and schemas. Delegated Domain Evidence is produced by an independent tool, hosted provider, adopter project, auditor, signing service, SBOM generator, provenance generator, or vulnerability scanner. AI Cockpit can require, bind, display, and archive delegated evidence; it cannot silently manufacture it.

<!-- section-id: commands-and-demonstration -->
## Commands and Demonstration

The offline failure-oriented demonstration is:

```sh
./docs/examples/trust-layer-demo.sh
```

The quality and lifecycle paths include:

```sh
make unsupported-claim-regression
make delusion-test-gate
make ai-preflight CONTRACT=.ai/work-items/active/<task>.contract.json
make ai-finish TASK=<task> REPORT_LANGUAGE=<conversation-locale>
make ai-close-work-item TASK=<task>
```

Commands are evidence only when their output, input commit, environment, and owning Work Item are recorded. The demonstration is offline and harmless; it does not simulate a hosted release or enterprise control.

<!-- section-id: boundaries-and-navigation -->
## Boundaries and Navigation

Use these documents according to their authority:

- [Design Philosophy](philosophy/design-philosophy.md) — North Star and design principles.
- [Architecture](architecture.md) — components, evidence ownership, and data flow.
- [Security and Release Verification](getting-started/security-release-verification.md) — release-level external evidence requirements.
- [Capability Truth Matrix](reference/capability-truth-matrix.md) — the only current implementation-status source.
- [Enterprise Control Checklist](reference/enterprise-control-checklist.md) — adopter and external-control responsibilities.
- [Documentation Architecture](reference/documentation-architecture.md) — the map of authoritative roles.

The README is the short entry point. This document is the complete Trust Layer authority. Neither replaces the tools and external controls that produce the evidence.
