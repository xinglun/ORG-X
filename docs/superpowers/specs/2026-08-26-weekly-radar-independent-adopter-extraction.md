# Weekly Radar Independent Adopter Extraction

## Problem

The merged-main Weekly Radar trigger found that Microsoft had two explicitly configured independent customer disclosures, PwC and Atos. PwC was counted, but the Atos disclosure was retained without a named adopter because its prose uses an infinitive deployment construction (`Atos Group becomes ... to deploy Microsoft 365 Copilot`). The independent diffusion gate therefore reported one source and correctly remained closed.

## Decision

Extend the existing bounded named-adopter extraction with one sentence-local construction: a capitalized adopter name followed by a bounded copular/change phrase and an explicit infinitive adoption or deployment verb. Preserve the existing direct-verb extraction, source-role mapping, evidence dates, source URI, and fail-closed judgment gate.

The matcher must remain deterministic and bounded:

- it operates only on substantive, dated, explicitly configured independent documents already admitted by the evidence pipeline;
- it recognizes only explicit adoption/deployment verbs already represented by the diffusion lexicon;
- it stops at sentence boundaries and limits the intervening phrase length;
- it does not add URLs, infer source ownership, or promote title-only material.

## Acceptance

- A deterministic Atos-shaped fixture fails before the code change because no named adopter is retained.
- After the minimal change, it yields `IndustryDiffusion`, named peer `Atos Group`, and `IndependentCustomerDisclosure`.
- PwC, supplier-attribution, title-only, and incomplete-gate regressions remain green.
- A merged-main dry-run reports two independent diffusion sources for Microsoft and no `independent_diffusion_sources` missing proof. Any remaining gate must remain visible, and no Ranking may be emitted unless the existing complete gate passes.
