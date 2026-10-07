# The executive — contract before code

This is the missing runtime layer. The lane *mechanisms* exist; the layer that
chooses between them, hands candidates between them, and guarantees an answer
does not. It is written first, in the project's own order (docs before
scanners, contracts before code), so the operator can redirect it before it is
built.

Sources it must honour, verbatim from the operator's own artifacts:

- `new-complete-atom-lightcone/docs/ARCHITECTURE.md` — "Reasoning lanes consume
  only admitted material. Each lane has its own typed intermediate steps,
  failure controls, and output contract. Creativity is a proposal generator:
  novelty cannot override hard constraints, evidence, causality, safety, or
  permission." … "**Bridges can hand candidates between these lanes, but do not
  merge their stages, controls, evidence classes, or scores.**"
- `atom-harness/contracts/atom-cognitive-engine-architecture.json` —
  `atom-cognitive-world-executive-v1`: fast causal proposal → multi-world
  deliberation → conflict resolution → evidence verifier; slow is
  authoritative; "unresolved conflict or insufficient support returns unknown";
  `language_model_boundary: translation only`.
- `atom-harness/contracts/atom-creative-probability-response.json` — the
  several-answers shape, with `winner_declared: false`,
  `factual_claim_authority: false`, `normalized_across_paths: false`, and a
  required uncertainty interval per path.
- The operator's orchestrator doctrine: "**the orchestrator is all policy and
  no mechanism**", and the shape is recursive.

## Authority

The executive is policy. It may select lanes, order rungs, hand candidates
across bridges, and decide that a rung failed. It may not invent an answer, a
cause, a source, or a confidence. It holds **no factual authority**
(`factual_claim_authority: false` on every output path), it never promotes a
creative candidate to causal truth (`causal_truth_claimed: false`), and it
never merges lane quantities into one score.

## The lanes

Mechanisms, one typed contract each, already implemented: the seven
`reasoning.*` lanes (deductive, inductive, abductive, analogical, causal,
counterfactual, probabilistic) and the seven `creativity.*` mechanisms. The
catalog also declares `thinking.*` (4), `analysis.*` (2), `geometry.structural`
and two `domain.*` lanes; those are **not** in this increment and are not
pretended to exist.

## Selection — policy, deterministic, no model

Inputs: the question's typed fields (goal, cause, effect, conditions) and the
admission's typed edges. Output: **one or more lanes**, never an argmax-only
choice, because the operator's rule is that several may run and their results
stay separate. Selection is pure and testable; the same inputs select the same
lanes.

## Bridges — hand-off without merging

A bridge carries a candidate from one lane to another. It carries the
candidate's provenance (link ids, source ids) and its evidence class. It
**never** carries the source lane's score, status, or controls: those stay with
the lane that produced them, which is what keeps the lanes from collapsing into
one retrieval score. A bridge fires only when the source lane produced a
candidate and the receiving lane's own contract accepts it.

## Dual process

- **Fast** — the highest-ranked selected lane supplies a **provisional
  proposal**. Labelled provisional.
- **Slow** — every selected lane runs, its output is verified against the
  admission, and conflicts are resolved. The slow result is **authoritative**.
- **Fail closed** — unresolved conflict or insufficient support returns
  **unknown**. Unknown is a real answer about the evidence, not a failure of
  the harness.

## Creativity — the guaranteed lane

Creativity is not causal and is not a reasoning lane. It activates when the
reasoning rungs produce no supported outcome, and its purpose is that **the
harness never fails to produce an answer or task**.

Its output is the contracted creative probability response:

- several paths, at least two when material allows, at most five;
- each path carries `probability_score` **and** a required uncertainty
  interval, `probability_kind:
  uncalibrated_probability_of_feasibility_if_prototyped`, `status:
  speculative_candidate_pending_prototype`, and a non-executing `choose_action`;
- `winner_declared: false` — no winner is declared;
- scores are independent and **not** normalized across paths;
- `factual_claim_authority: false`, `tool_execution_authorized: false`,
  `build_authorized: false`, `causal_truth_promotion_authorized: false`.

If the creative lane has no material either — an impossible brief, an empty
admission — the executive emits one **labelled speculative fallback
candidate** rather than an empty result. Never an empty result; never a
fabricated factual answer. Those two rules together are the guarantee.

## Separation of quantities (anti-collapse)

Reported side by side and never blended: per-lane status, per-lane direction,
per-lane evidence weight (where a lane emits one, always labelled
uncalibrated), bridge provenance, creative feasibility scores, and the
executive's own rung record. There is no aggregate confidence anywhere in the
output.

## Acceptance

- Selection is pure and deterministic; a test pins lane sets per typed input.
- Bridges never carry a score; a test pins that a bridged candidate's
  provenance survives and the source lane's status does not travel with it.
- Fast is provisional, slow is authoritative, and unresolved conflict yields
  `unknown`; each pinned by a test.
- A question no lane can serve still returns: creative paths, or the labelled
  speculative fallback. Pinned by a test.
- No output path sets `factual_claim_authority`, and no creative candidate
  claims causal truth. Pinned by a test.
- The line budget holds: every crate within 500 physical lines including tests
  and manifest, which is why this layer is its own crate rather than more lines
  in the reasoning engine.
