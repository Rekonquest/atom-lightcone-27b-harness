# New Complete Lightcone project rules

This repository is a from-scratch implementation. The earlier
`Atom-lightcone-27b-harness` may be inspected only as a read-only coverage and
failure record. Do not copy its trained worlds, feedback state, claim pack,
runtime code, binaries, or generated artifacts into this repository.

## Architectural invariants

1. The Lightcone is passive causal geometry. It admits an existing causal
   neighborhood and emits a receipt. It does not invent facts, compose an
   answer, issue a verdict, or learn model prose as truth.
2. World artifacts and their evidence are model-independent. Model-specific
   tokenization, prompting, logit access, tool syntax, and inference APIs live
   behind replaceable adapters.
3. Reasoning and creativity are explicit lanes. Deductive, inductive,
   abductive, analogical, causal, counterfactual, probabilistic, bottom-up,
   lateral, creative, philosophical, and rhetorical behavior must not be
   collapsed into one generic retrieval score.
4. Geometry is a foundational and cross-domain lane. Preserve exact graph
   reachability and geometric invariants independently of learned coordinates.
5. The domain registry is open-ended. User examples are cumulative minimum
   coverage, never an allowlist or completion boundary.
6. A trained result is not accepted without its source manifest, split
   manifest, configuration, seed, accelerator evidence, metrics, negative
   controls, artifact hashes, and sealed-test receipt.

## Evidence and completion

- Keep sourced observations, hypotheses, theories, interpretations, creative
  proposals, and executable/formal results in distinct epistemic classes.
- Preserve counterevidence, limitations, scope, version, and provenance.
- Split held-out mechanisms and cross-domain compositions before generating
  variants. Do not leak templates or paraphrases across the split boundary.
- Unknown and out-of-cone inputs are required negative controls.
- Never call the project, a domain, or a lane exhaustive while the coverage
  ledger contains a known gap, an untrained required item, a failed gate, an
  unverified artifact, or an untested supported adapter.
- Historical results are evidence records, not current verification.

## Execution boundaries

- Keep repository writes inside this workspace. Kaggle datasets, notebooks,
  and runs are allowed only for this project's training and validation.
- Kaggle TPU work must be real. Never substitute a local simulation while
  reporting TPU training.
- Build generated artifacts under `artifacts/`; keep immutable receipts under
  `receipts/`.
- Make the smallest change at the owning layer and add a regression test for
  every corrected boundary failure.

