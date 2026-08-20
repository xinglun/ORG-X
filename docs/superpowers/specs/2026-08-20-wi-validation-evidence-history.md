# WI Validation Evidence History Specification

## Purpose

Provide a deterministic, bounded retention contract for the validation strategy. The context records what was supplied at T0 and at later horizons; it does not decide whether a company advanced through ORG-X stages.

## Record shape

Each `ValidationRecord` contains:

- a baseline company identifier;
- supplied T0 stage text;
- baseline evidence references;
- hypotheses, counter evidence, missing proof, and peer-baseline metrics;
- zero or one observation for each of `SixMonths`, `TwelveMonths`, and `TwentyFourMonths`.

Each follow-up observation retains an opaque observation time, the five documented validation signals, measured metric values/units, source quality, and evidence references. Values are not parsed or arithmetically compared.

## Invariants

- required text cannot be blank;
- evidence references are unique within a record and cannot be duplicated in the same evidence set;
- metric names are unique within a baseline or observation;
- a record cannot contain two observations for the same horizon;
- an in-memory store cannot overwrite a company record;
- rejected additions do not mutate the record or store.

## Application boundary

`ValidationEvaluator` returns the missing horizons and a complete/incomplete readiness state. It does not calculate Stage, score, ranking, threshold distance, economic significance, or investment output. The context is not connected to the Weekly Radar runtime in this WI.

## Explicit limitations

This WI does not provide external validation data, schedule horizon jobs, authoritative S&P 500/Nasdaq 100 membership, production receipts, full runtime judgment-chain integration, or source-host security policy. Those require separate evidence and/or product decisions.
