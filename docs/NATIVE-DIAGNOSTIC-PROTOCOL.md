# Native post-weights diagnostic protocol (development only)

This is not the frozen restoration/effectiveness acceptance corpus. The
current four-crate Rust engine is an initial lane implementation, not the
original complete cognitive executive. Its path-count temperature response is
an experimental control, not a validated Atom reasoning or creativity uplift.

Identity: LM Studio Bonsai-2 Uncensored Heretic `PQ2_0` GGUF SHA-256
`c0959544f1422c569d07ccef8911dc0d016034bc1285d91010a6630fdf4c4dda`;
PrismML fork source `f40a7bf69ba1ac1c07608fd675606fa0e090efc3`,
copied source checksum-identical; local CUDA-enabled build and RTX 5070 Ti.
Lightcone pack `network-security-browser-v0/0.2.0` manifest SHA-256
`2588a0290eb3118206f0d8fbb427a2b025a74d8eb4ef6b55cba940606e175602`.

Revision 6 (2026-09-24). The placeholder pack `creativity-thinking-v0` (100 nodes,
manifest SHA-256
`876a84fe63b9193ada82015956e36a5de1e5a051cce7e5ea92936d8a0bd32aaa`)
was removed from the harness's world store; the store now holds only the sealed
cones. Every table and identity line below that names the placeholder is the
placeholder-era record and is kept as such. The commands in each `Reproduce:`
block were repointed at the sealed pack above so they still run.

Matched development arms: one fresh model context reset per arm; identical prompt bytes,
token budget, top-k 20, top-p 0.95, base temperature 1.0, fixed xorshift seed
`0x8d4942d5`. Atom arm alone dispatches the Rust engine on the first raw-logit
slice after `llama_decode` and before token selection. The current experimental
mechanism scales temperature by `1/(1+0.15*min(path_count,3))`; it uses no
preferred-token list or model-generated evidence. Bare arm never dispatches.
Unknown/no-path control must be byte-identical to bare. Record the receipt,
both raw texts, selected-token count and actual counterfactual deflections.
After the no-gain result below, this response was disabled by default; rerun
it only with `--experimental-temperature` after the token-budget argument.
Default native runs still execute the post-weights cognitive boundary, but
sample identically to bare until a validated intervention replaces this one.

Development cases, frozen before their first run under this sampler revision:

1. Direct admitted edge: query `Explain the relationship between candidate generation and creative selection in one sentence.`; cause `creative.generation`, effect `creative.selection`, goal `prove`.
2. Unknown seed: query `What does a nonexistent concept called zqx-777 mean? Answer in one sentence.`; cause `creative.generation`, effect `creative.selection`, goal `prove`. Expected zero admitted paths and byte-identical arms, not a fabricated factual answer.
3. Constraint path: query `Why are hard constraints applied before creative selection? Answer in one sentence.`; cause `creative.constraints`, effect `creative.selection`, goal `prove`. If the pack does not contain this path, preserve that unknown result rather than editing the pack or scorer.

These are diagnostics of wiring and failure modes, not evidence that the
harness improves a weak model. Effectiveness acceptance additionally requires
complete original-mechanism parity, a new unseen held-out corpus, finished
answers, source-grounded correctness scoring and protected regressions.

## Observed development runs, 2026-09-23

The standalone Prism runner exited 0 in all three cases. The direct edge used
receipt `ac121fb35acc9f711ba9534668b24c33929cd02c84c9ca27e633e40a8880e462`.
Bare generated 38 tokens; Atom generated 36 and had one counterfactual
selection change. Both produced an accurate one-sentence explanation. Bare:
"Candidate generation produces a diverse pool of potential options, which
creative selection then narrows down by applying evaluative criteria to
identify the most promising, feasible, or impactful solutions." Atom:
"Candidate generation produces a diverse pool of potential options, which
creative selection then narrows down by applying evaluative criteria to
identify the most promising ideas for further development." No gain is
established.

Unknown-term control receipt:
`56d320d47c0172720f6f3ec7d343158c01645b99ba432e568951bf4103ce11fe`.
Bare and Atom both generated 21 tokens with zero deflections and identical
text: "It means nothing, as it is a fabricated term with no established
definition or function." This is a wiring/unknown control, not a scored
factual-verification result.

Constraint question receipt:
`0eaf0bd61cdcbde192ac9ea14cc82aaaf5192c5e04e31955a47bfd3a97c848c0`.
The Lightcone admitted zero nodes and zero edges. Both arms generated 33
tokens, zero deflections, identical text: "Hard constraints are applied first
to eliminate impossible options, ensuring that the subsequent creative
process focuses only on viable solutions that meet non-negotiable
requirements." The model may know this independently; it was not grounded by
this admission. This case does not test an Atom improvement.

The current path-count temperature operation is rejected as an effectiveness
mechanism on these results. A live post-weights call and occasional token
deflection are real, but neither establishes cognitive benefit. Keep the
diagnostic code and failing/no-gain evidence while replacing this operation
with source-faithful cognitive execution and a newly frozen final comparison.

## Sampling-boundary binding revision (rev 3, 2026-09-24)

Revision 3 wires the boundary that `docs/EVALUATION.md` requirement 3 was
waiting for. The enforced order is the old harness's contract:

```text
Prism weights -> raw logits -> lightcone-direct-post-weights -> token selection
```

The admitted nodes' labels and aliases become a bounded, adapter-owned
preference set — lowercase words of at least four characters that encode to
exactly one token, sorted, deduped, capped at 64. A registered llama sampler
named `lightcone-direct-post-weights` applies it over the full-vocabulary
candidate array before this harness's own selection stage. It fires only when
the top-2 margin is under the adapter's uncertainty margin (1.5), and adds at
most `bias_cap` (0.75) to preferred tokens; confidence 1.0 and scale 0.45 are
adapter constants, never Lightcone output. The receipt hash is carried and
length-checked. Empty grounding — unknown seed, out-of-cone query, or a pack
that admitted nothing — builds a pass-through sampler that cannot change one
logit.

Attachment note, deliberate and different from the reference: the old harness
put this filter in a llama sampler chain followed by greedy. This harness keeps
its own selection stage (top-k 20, top-p 0.95, base temperature 1.0, fixed
xorshift seed) so both arms continue to share one selection stage; the filter
is applied through `llama_sampler_apply` at the same boundary position, with the
same math and the same bounds.

Runs on 2026-09-24, pinned model, pinned pack, budget 128, one fresh context per
arm, binding enabled by default:

| Case | Receipt | Preferred tokens | Raw-logit passes | Fires | Deflections | Arms |
| --- | --- | --- | --- | --- | --- | --- |
| 1 direct admitted edge | `ac121fb3…0e462` | 6 | 34 | 15 | 1 | differ |
| 2 unknown seed (`zqx-777`) | `56d320d4…3e11fe` | 0 | 22 | 0 | 0 | byte-identical |
| 3 constraint path (0 admitted) | `0eaf0bd6…848c0` | 0 | 34 | 0 | 0 | byte-identical |

Case 1 atom text: "Candidate generation produces a diverse pool of potential
options, which then serves as the foundational input for creative selection to
identify and refine the most compelling ones." The six preferred tokens are
exactly the single-token words of the three admitted nodes' labels and aliases
(candidate, generation, creative, proposal, system, selection); no other
vocabulary entered the boundary.

Parity with the previous revision, same case, binding disabled
(`--no-lightcone-tokens --experimental-temperature`): bare 38 tokens, atom 36
tokens, one counterfactual deflection, and the recorded revision-2 atom text
byte-for-byte. Introducing the binding therefore changed no existing path.

Reproduce:

```bash
runtime/native-harness/target/debug/native-harness \
  /home/jesse/.lmstudio/models/OS-Software/Ternary-Bonsai-2-27B-Uncensored-Heretic-GGUF/Ternary-Bonsai-2-27B-Uncensored-Heretic-PQ2_0.gguf \
  lightcone/artifacts/worlds/network-security-browser-v0/0.2.0 \
  "Explain how a TCP acknowledgment leads to retransmission in one sentence." \
  net.tcp-ack net.retransmission prove 128
```

What this establishes, and what it does not. It establishes that a real passive
admission binds the exact pack and receipt at the model's sampling boundary,
that the influence is bounded and gated, and that unknown and out-of-cone
controls stay exact pass-through. It establishes no model-performance
improvement: one deflection in one development case is not uplift, and the
no-gain result above for the path-count temperature operation remains evidence.
The final held-out corpus and the original-mechanism parity gates are still
open.

## Structured causal-model translation experiment (separate development arm)

This is **not** the post-weights temperature test and is not a final held-out
effectiveness corpus. The arm is opt-in with
`--experimental-structured-translation --law <cause> <effect> <domain>` after
the token budget. After the first decode exposes raw logits, the Atom
sourced-law executive executes before token selection. If it returns a
supported path, the host restarts the model context with a typed causal-model
artifact (feature path, conditions, law and evidence IDs, bounded confidence
and persistence) and the same user query. The artifact is explicitly marked
not an observed factual verdict. Both arms otherwise use top-k 20, top-p
0.95, temperature 1.0 and the same seed. This tests translation of executed
structure; it is not a Lightcone/CausalLaw join.

Development cases frozen before the first translation run:

1. Source-specific: query `According to the installed explanation law, list its exact context keys in one sentence.`; law cause `philosophy:explanation_phenomenon`, effect `philosophy:explanation_explanatory_hypothesis`, domain `symbolic`. Expected keys are `inquiry:general` and `philosophy_branch:explanation`; both must appear and no other context key may be invented.
2. Unknown-law control: same query with cause `absent:feature`, effect `absent:effect`, domain `symbolic`. Expected no structured translation, byte-identical arms and no invented law ID.

Passing these cases would show only exact local evidence translation and
fail-closed unknown behavior. It would not establish broad model-performance
improvement; the final held-out and original-mechanism parity gates remain.

### First translation result and explicit revision

At 96 tokens the bare arm invented keys `context`, `key`, `value`,
`description`. The grounded arm received the correct source keys but emitted
only `<think>` text and no final answer. A 256-token retry still produced no
released answer; neither run passes. The unknown-law control returned
`active_world_search`/`Unknown`, used no structured translation, and the two
arms were byte-identical. Preserve these failures.

Revision 2 (development, not held-out): use the Prism/Qwen3.5 chat template's
non-thinking assistant continuation in **both** arms with `--no-think`; shorten
the artifact to law ID, direction, feature path, exact per-law context keys,
evidence count, confidence and persistence. The expected keys and unknown
control above are unchanged. Test answer release and exact keys before any
new mechanism or final-corpus claim.

Revision 2 result (96-token budget): the source-specific case released a
complete Atom-arm answer naming both exact keys, `inquiry:general` and
`philosophy_branch:explanation`, with no additional context key. It cited
`law:01e07c02cafea26c28d59aac`. The bare arm instead said no such installed
law or keys exist. The Atom text included raw nested-array brackets, so the
rendering is awkward but the exact-key development check passes. Both arms
used the same non-thinking template and sampler; the intervention was the
post-weights-triggered context restart, not token bias. The unknown-law
control returned `Unknown`, performed no restart and produced byte-identical
48-token answers in both arms. This is a narrow local evidence-translation
success, **not** general weak-model improvement or original-harness parity.

## Closed-membrane revision (rev 4, 2026-09-24)

Revisions 1–3 covered the input side and the sampling boundary. Rev 4 closes the
membrane the way the original did — four lanes — and adds the consumers the
second required boundary names.

The lanes, in the enforced order:

1. **In** — a passive admission binds the pack and the receipt (`harness-cli`),
   unchanged from revision 1.
2. **Weights** — `lightcone-direct-post-weights` over raw logits (rev 3),
   unchanged.
3. **Out** — engine-side, in `crates/atom-orchestrator/src/boundary.rs`. The
   admitted claim's polarity is compared with what the text asserts:
   `Consistent`, `Contradicts` (withheld — the wrong verdict does not cross),
   `Ungrounded` (asserted but not covered by this admission), or
   `NotApplicable`. The marker lists are explicit and the check reports which
   markers fired, so the instrument's coverage is visible rather than assumed.
   Claim polarity, evidence class, the engine's own lane status, and the release
   decision are separate fields; none is blended, and `factual_claim_authority`
   is false on every path. Verdicts live here because this is the Atom side
   acting through the boundary — the Lightcone still holds none.
4. **Feedback** — verified claims reinforce a bounded, persistent warrant ledger
   (`.runtime/lightcone-boundary/verdicts.json`, schema 1, limit 1024, which
   refuses new claims rather than evicting old ones). Only the grounded arm
   reinforces: the bare arm may use the same claim out of the model's own
   knowledge, and crediting the engine for that would be unearned uplift. The
   claim's epistemic class is stored beside its count, never merged into it.

Consumers, the other half of required boundary 2: the seven creative mechanisms
now dispatch inside the runner under `--creative <brief.json>` — a bounded,
host-supplied brief — grounding their proposals in the reasoning paths' source
IDs. Per-lane evidence weights, the prediction relation, and contested-evidence
status are consumed and reported separately; no blended score is computed and no
probability is promoted to factual confidence.

Runs on 2026-09-24, pinned model and pack, corpus
`runtime/native-harness/fixtures/development-corpus.json` revision
`development-2026-09-24`, one fresh process and one fresh context per arm, one
per-case ledger:

| Case | Class | Verdict | Admitted links | Reinforced | Arms | Fires | Deflections |
| --- | --- | --- | --- | --- | --- | --- | --- |
| direct-admitted-edge | grounded | Consistent | 2 | yes | differ | 15 | 1 |
| unknown-term-control | unknown | NotApplicable | 0 | no | byte-identical | 0 | 0 |
| out-of-cone-control | out-of-cone | Ungrounded | 0 | no | byte-identical | 0 | 0 |

The out-of-cone verdict names its own marker (`ensuring`): the text asserted a
directed relation this admission did not cover. That is this protocol's earlier
prose note — the model may know it independently, it was not grounded here —
expressed as a check instead of a paragraph.

Restart proof: two separate processes on the grounded case took the warrant
count from 1 to 2 with no other state carried over.

Reproduce the whole membrane:

```bash
runtime/native-harness/target/debug/corpus \
  /home/jesse/.lmstudio/models/OS-Software/Ternary-Bonsai-2-27B-Uncensored-Heretic-GGUF/Ternary-Bonsai-2-27B-Uncensored-Heretic-PQ2_0.gguf \
  lightcone/artifacts/worlds/network-security-browser-v0/0.2.0 \
  runtime/native-harness/fixtures/development-corpus.json \
  --creative runtime/native-harness/fixtures/creative-brief-development.json
```

One bug found by the live run and fixed: the creative-brief loader read `frames`
and `tensions` from the document root while the fixture — and the engine's own
`Brief` type — nests them inside `brief`. A fixture's tension therefore produced
no bridge, and the unit test had not asserted that it arrived. The parser now
reads them where they belong and the test asserts the bridge reaches a
candidate.

What rev 4 establishes, and what it does not. It establishes that all four lanes
run in one process over one admission, that the release decision and the
feedback write follow from the engine's own classification, that the seven
creative mechanisms run in the runner, and that the unknown and out-of-cone
controls stay exact pass-through and reinforce nothing. It establishes no answer
quality, no correctness score, and no model-performance improvement: the
development corpus gates wiring and failure modes, the held-out corpus and
correctness scoring are not frozen, and the original-mechanism parity gates
remain open.

## The executive in the loop (rev 5, 2026-09-24)

Revisions 1–4 wired the membrane: input admission, the post-weights binding, the output
lane, and verified-claim feedback. Rev 5 puts the **executive** between the lanes and the
model, so the harness chooses lanes, hands candidates between them, and never fails to
produce an answer.

- `crates/atom-executive` — policy only: deterministic multi-lane selection with an
  auditable reason per lane, one dispatch per lane contract, an authoritative aggregation
  where a cross-lane conflict fails closed to unknown and is reported, bridges that carry
  provenance and evidence class and — by construction — no status or score, and the
  guarantee that a result is never empty.
- `crates/atom-creative-paths` — the contracted creative probability response, in the old
  harness's own formula and labels (`uncalibrated_probability_of_feasibility_if_prototyped`,
  no winner, scores independent rather than normalized, choice records intent only).
- Two verification leaves (`atom-lane-contracts`, `atom-executive-contracts`) hold the
  contract tests; that is how every crate stays inside the 500-line build contract.

Runner changes. The grounded arm dispatches through the executive instead of the raw
engine. `--creative <brief.json>` now *supplies material* to the executive rather than
forcing a dispatch — the executive decides whether the creative lane runs, which is the
operator's design. `--reasoning-brief` (new, off by default) renders the executive's answer
into the model's context, so the model reasons from the lanes instead of guessing.

Verified live on 2026-09-24, pinned model and pack, corpus revision
`development-2026-09-24b`: **9 of 9 cases pass**, all seven lanes exercised, and the
never-fail rule is demonstrated on the case that previously answered with nothing behind it.

| Case | Rung | Status | Creative paths | Reinforced |
| --- | --- | --- | --- | --- |
| direct-admitted-edge | Slow | SupportedPath | 0 — reasoning answered | yes |
| out-of-cone-control | SpeculativeFallback | Unknown | 1 labelled speculative | no |

The out-of-cone answer carries `status=speculative_candidate_pending_prototype`,
`kind=uncalibrated_probability_of_feasibility_if_prototyped`, `score=0.050000`,
`band=(0.000,0.210)`, `proposal_only=true`, `factual_claim_authority=false`, the contract's
disclaimer, and no declared winner. The output lane still reports `Ungrounded` and nothing
is reinforced: a proposal was produced, no fact was claimed.

What this does not establish: any answer quality or model improvement. The corpus gates
wiring and failure modes; the held-out corpus, correctness scoring, and the
original-mechanism parity gates remain open.

## Revision 6: the sealed cones replace the placeholder (2026-09-24)

The placeholder pack `creativity-thinking-v0` (100 nodes, 129 edges) was removed
from `lightcone/artifacts/worlds/`; the store now holds the sealed cones only
(177 pack directories, every one with its own manifest). Defaults, fixtures and
unit tests were repointed at `network-security-browser-v0/0.2.0` (353 nodes, 305
established, 200 non-membership edges; manifest SHA-256
`2588a0290eb3118206f0d8fbb427a2b025a74d8eb4ef6b55cba940606e175602`). The
placeholder-era corpus is kept verbatim as
`runtime/native-harness/fixtures/development-corpus-placeholder-era-2026-09-24b.json`;
its cases named vocabulary (`creative.generation`, `nonlinear.feedback`) that no
longer exists anywhere in the store.

What the sealed cone changes in the run, measured with `harness-cli` (engine
only, no model) on 2026-09-24:

| case | admitted | lanes reported | statuses |
| --- | --- | --- | --- |
| `net.tcp-ack` to `net.retransmission`, prove | 3 nodes / 2 edges | Deduction | SupportedPath |
| `net.queue` to `net.latency`, generalize | 4 / 3 | Induction | Unknown |
| `net.wireless-interference` to `net.loss`, explain | 3 / 2 | Abduction | Provisional |
| `net.queue` to `net.loss`, causal | 4 / 3 | Causal, Deduction | Unknown / SupportedPath |
| `net.tcp-ack` to `net.retransmission`, counterfactual minus `nsb.relation.net-ack-retransmit` | 3 / 2 | Counterfactual | Provisional |
| `net.http-request` to `net.http-response`, transfer | 6 / 5 | Analogy | Provisional |
| `net.queue` to `net.loss`, estimate | 4 / 3 | Probabilistic | Unknown |

Two observations that are their own work item:

1. The sealed nodes declare their own lane membership and nothing in this
   harness reads it. In this pack `reasoning.causal` is declared on all 353
   nodes, `thinking.bottom_up` on 353, `reasoning.probabilistic` on 249,
   `geometry.structural` on 217, `reasoning.counterfactual` on 136. The
   lightcone crate's own tests exercise the field; no Rust consumer in the
   harness consults it, so lane selection stays goal-keyed only.
2. Lane preconditions were tuned on the placeholder's schema. On the sealed
   cones the causal goal is answered by the deduction lane while the causal
   lane stays Unknown even when a real `causes` edge (`nsb.relation.net-queue-loss`)
   is admitted; Induction and Probabilistic likewise report Unknown over
   admitted edges. No lane reported a weight anywhere, so nothing was invented:
   this is a precondition gap, not a calibration leak.

Query shape matters and is now a rule for every fixture: the admission drops an
edge whose target is already a seed (reproduced 2026-09-24), so each corpus
query names the cause side only.

### The gates on the sealed cone (2026-09-24)

`correctness` against `correctness-flagship.json` on
`network-security-browser-v0/0.2.0` (log
`.runtime/receipts/sealed-pack-gates.log`, exit 0):

| case | admitted | lane paths | rung | bare arm | atom arm |
| --- | --- | --- | --- | --- | --- |
| `enables-established` (connected) | 2 links | `[1, 1]` | Slow | WRONG: "answer asserts no relation between them" | WRONG: same |
| `unconnected-novel` (control) | 0 | `[0, 0]` | SpeculativeFallback, 1 creative path | CORRECT | CORRECT |

Score: bare 1/2, atom 1/2; grounding changed nothing on this corpus. The control
case shows the never-empty mechanism working end to end on the sealed cone: no
admission, speculative fallback, correct abstention, and the creative path count
is reported rather than folded into a score. The connected case is the open
defect: the cone admits the real enabling edge (`nsb.relation.net-ack-retransmit`,
established, declared condition) and the answer still denies the relation, so
the post-weights grounding does not yet move the text on this pack. Two cases
cannot tell a broken mechanism from an unhelpful one, so this is recorded as an
observation, not a verdict.

The wiring gate (`corpus`, development-corpus revision c) is recorded in
`.runtime/receipts/corpus-sealed-revc.log`.

### The wiring gate on the sealed cone (2026-09-24)

`corpus` against development-corpus revision c, 9/9 per class, exit 0
(`.runtime/receipts/corpus-sealed-revc.log`):

| case | verdict | reinforced | arms identical | fires | deflections |
| --- | --- | --- | --- | --- | --- |
| deduction-direct-edge | NotApplicable | false | true | 16 | 0 |
| induction-single-cone | Consistent | true | true | 17 | 0 |
| abduction-rival-causes | Consistent | true | false | 45 | 0 |
| causal-trace | Contradicts | false | true | 55 | 0 |
| counterfactual-removed-link | NotApplicable | false | true | 16 | 0 |
| analogy-transfer | NotApplicable | false | false | 42 | 1 |
| probabilistic-no-weight | NotApplicable | false | true | 53 | 0 |
| unknown-term-control | NotApplicable | false | true | 0 | 0 |
| out-of-cone-control | Ungrounded | false | true | 0 | 0 |

Read together with the correctness gate, three things are established and one
thing is not:

* The sealed cone drives the membrane: every case admits real geometry, the
  post-weights hook fires on every case that has an admission, and both controls
  admit nothing and fire nothing.
* The never-empty mechanism holds on a real cone: the unconnected control
  reaches the speculative rung with one reported creative path and its answer is
  scored correct.
* Nothing is invented: no lane reported a weight anywhere, and the two cases
  whose answers were checked against the cone's own edges (the connected
  correctness case, and `causal-trace`) both came back negative, which is a
  failure to ground rather than a fabricated claim.
* Not established: that the cone changes the text. The arms are byte-identical
  in seven of nine wiring cases, and identical on the connected correctness case,
  even though the hook fires. So the loop admits, fires, and leaves the answer
  where it was — except where the abductive and analogical machinery engages.

Revision d of the corpus freezes those observations as expectations, with fire
counts asserted at-least-one rather than as frozen integers; its run is recorded
in `.runtime/receipts/corpus-sealed-revd.log`.

### Revision 6 errata after the NAT adversarial review (2026-09-24)

Fourteen concurrent review slices ran against this revision. Their findings that
correct this document are recorded here rather than edited away:

1. **A failed run sits inside the log this document calls exit 0.** The first
   corpus invocation at 17:23:43 died on `unsupported corpus schema 0` (the
   fixture had lost its `schema` field) and only the later runs at 17:29 and
   17:33 passed. `.runtime/receipts/sealed-pack-gates.log` carries that failure;
   the 9/9 lives in `corpus-sealed-revc.log` and `corpus-sealed-revd.log`.
2. **The weight-absent check was inert.** Revisions c and d supplied
   `lane_weight_absent` as an object; `runtime/native-harness/src/bin/corpus.rs`
   parses an array of lane names, so the check evaluated zero lanes and could not
   fail. "No lane reported a weight" was therefore ungated. Revision e of the
   corpus restores the array form; the gate is real again from that revision on.
3. **The lane abstentions are structural, not placeholder-tuned.** The cause is
   in the adapter: `runtime/harness-cli/src/lib.rs` sets `confidence: None` on
   every link it builds, and the causal and probabilistic lanes fold over
   `path.strength` (`crates/atom-reasoning-engine/src/lib.rs`), so an empty fold
   leaves both permanently `Unknown` over admitted `causes` edges. Induction
   cannot fire either: every link in one admission carries one `world_id`, and
   the lane requires two independent worlds. The earlier explanation in this
   revision ("preconditions tuned on the placeholder schema") was wrong.
4. **"No Rust consumer reads the declared lanes" is overstated.** The lightcone
   loader parses and validates the field; what no consumer does is read it for
   lane *selection*, which stays goal-keyed.
5. **The copy count was wrong.** Twenty-eight directories name the removed pack
   project-wide, not twenty-two; four carry a manifest, the rest are derived
   outputs (smoke, staging, kaggle).
6. **The store is not 178 sealed packs.** `lightcone/artifacts/worlds/` holds 177
   world directories and 355 versioned pack manifests; the flagship's seal
   receipt counts 178 worlds, and about 150 of the 177 directories are taxonomy
   meshes that no case exercises. The store also still carries a live ledger
   entry for the deleted pack (`.runtime/lightcone-boundary/verdicts.json`,
   claim `relation.generation-selection`, four placeholder-era reinforcements)
   and coverage-closure nodes whose `version_boundary` names it.
7. **The corpus revisions c to d are re-observation, not verification.**
   Revision d's values were taken from revision c's own run, and no receipt
   records the fixture's or the pack's hash, so a run cannot certify which pack
   it measured. An independent gate needs expectations authored outside the
   harness and hash-pinned before the run.
8. **The correctness instrument is narrower than the score suggests.** Its
   unconnected branch never reads the answer, so an empty or fabricated answer
   scores as correct there; its connected branch scores on a 16-phrase marker
   list, and no answer texts are recorded, so "did not state the relation" and
   "stated it without a marker" are indistinguishable. The development
   correctness fixture also still named the deleted pack's vocabulary and has
   been archived as `correctness-corpus-placeholder-era-2026-09-24b.json`.
9. **The token-bias channel is arithmetically weak.** The boost is
   `scale 0.45 x confidence 1.0 x urgency <= 1`, so the `bias_cap` of 0.75 can
   never bind; `preferred_tokens` sorts by token id and truncates to 64, keeping
   high-frequency words over distinctive domain terms; the prompt-side
   `--reasoning-brief` channel, which is the larger lever, is never passed by
   either gate. The pack's own trained retrieval geometry
   (`training/evidence-retrieval.json`, relation coordinates) is loaded by
   nothing.

Two defects that were fixed as a result of this review, with the fix in the
tree:

* **Never-empty was false on the error path.** A counterfactual question naming
  a link absent from the admission made the engine raise and the harness exit
  non-zero (`reproduced 2026-09-24`:

  ```
  harness-cli ... counterfactual not.in.this.pack
  Error: "Atom engine rejected input: Invalid(\"counterfactual link absent from admission\")"
  rc=1
  ```

  ). The executive now skips a refusing lane with the refusal recorded as its
  reason, and its creative rung falls back to the labelled speculative candidate
  instead of propagating; `crates/atom-executive-contracts/tests/executive.rs`
  pins both.
* **The inert weight check** in point 2 above.

## Revision 7: the reference lane is wired and measured (2026-09-26)

The prompt-side channel this harness was built with (`--reasoning-brief`) was
finished and turned on for the gates.

1. The brief now carries the admitted geometry itself. `render_geometry()`
   (`runtime/native-harness/src/reasoning_brief.rs`) renders every admitted
   typed claim with labels, relation kind, polarity, evidence class, declared
   conditions and source ids — declarations with provenance, bounded at 1 KiB,
   rendered ahead of the lanes artifact. The nodes' prose statements are
   deliberately not rendered: the reference lane carries the cone's typed
   claims, not prewritten answers, and nothing in the section is an instruction
   (Rule 1's compatibility boundary — passive geometry remains passive).
2. Both gates take `--brief`, which passes `--reasoning-brief` to the runner's
   atom arm. The bare arm never sees the artifact, so the paired design holds.
3. The correctness instrument was repaired where the NAT review found it blind:
   the unconnected branch now fails an empty answer (abstaining on the claim
   does not license producing nothing), both arms' texts are recorded (first
   160 characters, whitespace-flattened), and the receipt prints the pack path
   plus the pack manifest hash the runner computed.

Measurements (Bonsai-2 27B PQ2_0, RTX 5070 Ti, fixed seed, pack
`network-security-browser-v0/0.2.0`, manifest `2588a029…`):

| gate | configuration | bare | atom |
| --- | --- | --- | --- |
| correctness-flagship, 2026-09-24 | brief off | 1/2 | 1/2 |
| correctness-flagship, 2026-09-26 (`.runtime/receipts/correctness-brief.log`) | brief on | 1/2 | **2/2** |
| development corpus rev f | brief on | — | 9/9, reproduced twice byte-identically |

The connected case flipped to CORRECT with the brief on: the cone admits the
enabling edge, the brief carries the claim with its provenance, and the atom
arm's own reasoning shows it reading the admitted claims before answering. The
unconnected control stayed correct on both arms with zero admission and the
speculative rung. `causal-trace` moved `Contradicts` to `Consistent` under the
same configuration, closing the defect revision d froze.

What this establishes, and what it does not. It establishes that the reference
lane participates: the cone's admitted claims, rendered as typed evidence,
change the answer in the intended direction on the one case that measures it,
without the control degrading. It does not establish model uplift — two cases
cannot; the atom arm receives material the bare arm does not, which is the
mechanism working, not the model reasoning better. The scores remain counts of
cases right.

Anomaly recorded, not smoothed over: the first brief-on corpus run reported
`reinforced=false` under four `Consistent` verdicts, which is inconsistent with
`reinforces()` by construction and was contradicted by the second corpus run
and by two independent single-case reproductions with the exact bin invocation
(`reinforced=true`, byte-identical texts). The run is kept at
`.runtime/receipts/corpus-brief-revf-first.log`; the cause is unknown.

Housekeeping closed with this revision: the stale placeholder-era warrant was
removed from `.runtime/lightcone-boundary/verdicts.json` (both gates use
per-case temp ledgers; the runner recreates the resident ledger empty on next
load); and the store count is reconciled — the flagship's validation receipt
counts 178 worlds *including* the placeholder (its own entry,
`run_id creativity-thinking-v0-20260921T161805Z`, is in the receipt), so the
harness's 177 directories are the seal's 178 minus the pack the operator
ordered removed.

Reproduce the brief-on gates:

```bash
runtime/native-harness/target/debug/correctness <model.gguf> \
  lightcone/artifacts/worlds/network-security-browser-v0/0.2.0 \
  runtime/native-harness/fixtures/correctness-flagship.json --brief
runtime/native-harness/target/debug/corpus <model.gguf> \
  lightcone/artifacts/worlds/network-security-browser-v0/0.2.0 \
  runtime/native-harness/fixtures/development-corpus.json \
  --creative runtime/native-harness/fixtures/creative-brief-development.json --brief
```

## Revision 8: the admission defect is fixed and the causal lane answers (2026-09-26)

Three mechanical corrections, each verified before the gates were re-run:

1. **The seed-drop admission defect is fixed** in this harness's lightcone copy
   (`lightcone/crates/lightcone-core/src/admission.rs`). An edge whose target is
   already in the cone is now recorded as a traversed edge instead of being
   dropped; the node is still not re-queued, so the cone's shape is unchanged.
   Verified live: a query naming both ends (`TCP acknowledgment and
   retransmission`) now admits `nsb.relation.net-ack-retransmit`, which the
   defect had been silently losing since it was reproduced on 2026-09-24. The
   lightcone crate's own 44 tests pass unchanged. **The flagship copy at
   `new-complete-atom-lightcone` still carries this defect**; fixing it there is
   the operator's call, and until then this harness's copy is the fixed one.
2. **The causal lane answers without invented calibration**
   (`crates/atom-reasoning-engine/src/lib.rs`). Direction and magnitude are
   independent quantities: the pack declares a polarity and an evidence class
   but no strength, and the lane had been folding over the empty strength
   iterator, reporting `Unknown` for every uncalibrated cone. An established
   path with no declared strength now reports `Provisional` with the pack's own
   declared direction and `probability: None` ("declared direction without
   declared calibration"); signed conflict among established paths still fails
   closed; the calibrated fold is untouched for packs that do declare
   strengths. The probabilistic lane is deliberately unchanged — without
   declared calibration it has nothing to state, and stating nothing is its
   honest answer.
3. **Receipts are pinned.** Both gate bins print a corpus integrity key
   (FNV-1a 64-bit over the fixture bytes, labelled non-cryptographic — an
   anti-drift key, not tamper-proofing), the byte length, and the pack path;
   the correctness receipt already carried the runner's computed pack manifest
   hash. Two runs of the same fixture now print the same key, and a fixture
   edit is visible in the receipt.

Re-gated results (Bonsai-2 27B PQ2_0, fixed seed, brief on, pack
`network-security-browser-v0/0.2.0`, manifest `2588a029…`):

| gate | result | receipt |
| --- | --- | --- |
| correctness flagship | bare 1/2, **atom 2/2** | `correctness-brief-rev8.log` |
| development corpus rev g | 9/9, twice, byte-identical | `corpus-revg-a.log`, `corpus-revg-b.log` |

The richer cones from fix 1 are visible in the wiring numbers: admitted links
rose across the cases (for example the grounded case 2 -> 3, induction 3 -> 5)
and counterfactual deflections appeared where the geometry now reaches them
(abduction 2, analogy 1, estimation 1). `causal-trace`'s Causal lane reads
`Provisional` — the change revision g freezes deliberately, since it is the
corrected lane semantics and not a drift.

What remains open after this revision: the flagship copy still carries the
seed-drop defect; the probabilistic lane stays `Unknown` on uncalibrated cones
by design; the pack's trained retrieval geometry
(`training/evidence-retrieval.json`) is still loaded by nothing; and corpus
expectations remain authored from observed runs — the integrity key makes that
visible, it does not make it independent.
