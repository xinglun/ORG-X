# WI-006 Organization Evidence Domain

## Boundary

The Organization Domain retains management commitment, responsibility,
budget, decision-right, and organization-adaptation facts that may support
review of a changed production system. `OrganizationEvidence` groups those
facts for one organization while preserving insertion order and deterministic
duplicate-identity errors.

The boundary is descriptive and evidence-oriented. It does not enforce a
permission, allocate a budget, perform a management decision, or infer a
transformation stage or score. An adaptation keeps an opaque production-system
target reference so a later composition boundary can relate facts without
coupling this Domain to another Feature module.

## Decisions

- Required identities, names, statements, descriptions, amounts, units, and
  scopes reject blank input at the boundary.
- Amounts remain opaque strings; this WI does not perform arithmetic or
  allocation.
- Commitment authors, responsibility owners, decision-right holders, and
  adaptation targets are preserved as supplied descriptions.
- Organization facts are evidence and cannot independently upgrade a Stage.
- The implementation uses only the Rust standard library and imports no other
  feature module.

## No-goals

- Evidence acquisition, source adapters, persistence, scheduling, reporting,
  Telegram delivery, scoring, ranking, or stage transitions.
- Runtime enforcement of budgets, permissions, responsibilities, or management
  decisions.
- Trading, price prediction, capital-action behavior, or external operations.

## Authorization and issue policy

This WI is executed under the user's explicit authorization: `完成24 个WI，需要我授权的，授权给你并请写入Contract。`
The authorization is recorded in the Contract. If verification finds an issue
inside this organization-evidence boundary, it is resolved in WI-006; a
successor is reserved for a distinct boundary or a material scope expansion.

## Verification

- `cargo test --test organization_domain`
- `cargo test --all`
- `make check`
- `make ai-finish TASK=wi-006 REPORT_LANGUAGE=zh-CN`
