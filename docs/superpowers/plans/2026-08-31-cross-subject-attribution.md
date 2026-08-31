# 2026-08-31 Cross-subject Attribution

> Execute in the isolated Work Item worktree after the Contract is preflighted.

## Goal

Keep customer and partner claims under their real subject while ensuring only
same-subject claims can become Structural Evidence for the assessed company.

## Production regressions

Use the exact production passages from `origin/data:weekly-radar/reports/2026-08-31.md`:

- Hertz: `As part of its technology modernization strategy, Hertz has begun developing low-code applications and agents with Power Platform.`
- PwC: `PwC’s Engineering and AI builders help organizations modernize core platforms and translate AI, cloud, and data innovation into scalable business outcomes.`
- Atos: `Atos Group becomes the first French Global System Integrator to deploy Microsoft 365 Copilot and one of the largest to roll out secure agentic AI across its workforce.`

Each source observation is assessed as Microsoft, but the resulting subject
must be Hertz, PwC, or Atos Group respectively. Each must remain a
`ValidatedFact`, with no Structural Evidence dimension or contract.

## Red/green sequence

1. Add focused regressions for the three exact passages and a malformed
   external-subject structural contract. Run the focused test and record RED.
2. Implement deterministic direct-subject extraction for lead sentences,
   including possessive `PwC’s` and multi-word `Atos Group` forms. Use that
   attribution for promotion regardless of reference-model family detection.
3. Make the model/contract boundary fail closed when structural attribution is
   external; preserve the evidence as a validated fact.
4. Run focused GREEN tests, then formatting, clippy, and the locked workspace
   suite.

## Acceptance evidence

- Hertz, PwC, and Atos retain external subject attribution and never enter
  Microsoft Structural Evidence.
- Same-subject Microsoft production changes retain a valid complete contract.
- Runtime `preflight`, `verify`, `finish`, `archive`, finalization, and close
  receipts are recorded before the reviewed PR is merged and cleaned up.

## Stop conditions

Stop if subject extraction is ambiguous, if a cross-subject claim can still
obtain a structural dimension, if same-subject structural evidence loses any
contract field, or if a required verification gate fails.
