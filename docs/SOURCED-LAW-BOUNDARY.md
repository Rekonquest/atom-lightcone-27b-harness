# Copied CausalLaws: separate sourced-law boundary

On 2026-09-23 the user authorized copying the original harness's causal-law
world data from `/home/jesse/Desktop/Projects/atom-harness` into
`reference-causal-worlds/`. The original checkout was not modified. Thirteen
copied files passed checksum comparison against that checkout; their pinned
SHA-256 digests are in `reference-causal-worlds/MANIFEST.json`.

The five installed world-model payloads passed the original Python
`CausalGraph.from_model_payload` check against the copied bytes (652 laws
total). That Python invocation was a read-only compatibility check, not an
Atom runtime. The archival `causal_world_outputs/atom_causal_world_model.json`
failed the same original loader with `causal graph payload is not an exact
runtime round trip`; retain it for provenance but do not activate it.

`atom-causal-laws` now preserves CausalLaw identity, signed direction,
confidence, persistence, support, condition counts, provenance hashes and
evidence IDs as a type distinct from Lightcone's admitted edges. The host
loader pins both raw-file SHA-256 and the embedded original model hash before
constructing the Rust registry. It rejects malformed and ungrounded
crystallized laws. The Rust path search is bounded to depth 8, 64 paths and
8,192 explored edges; the executive performs initial task-driven world routing
and signed conflict resolution. This is a partial mechanism port, not
original-executive parity: phase mixing, full within-world CausalCognition,
resident learning, Mother context, and all original benchmark gates remain
open.

Tests on copied data load all five active packs and derive the direct
philosophy law `law:01e07c02cafea26c28d59aac`; wrong-domain and unknown
queries return no path. A separate explicitly synthetic fixture confirms a
two-world path still executes when no direct world routes, without presenting
that fixture as real evidence.

The native Prism runner also executed the sourced-law engine on the first
raw-logit slice for a real Bonsai-2 turn. It reported a positive sourced-law
path with confidence `0.6825383702440335`, while the passive Lightcone pack
admitted zero nodes for that prompt. The two model arms were byte-identical,
and the runner reported `lightcone_sourced_law_typed_join=false`. This is
proof of separate live execution, not a useful joined intervention or an
effectiveness gain.

Do not infer a cross-ontology warrant from word overlap. For example, the
Lightcone `philosophy-rhetoric-v0/0.2.0` receipt
`0e9858bfc141ac286faaec13827046744b0c1f0bc495cba3cdde79860bc789c4`
contains `p.hypothesis -> p.explanation` as an `enables` edge with a fixed
context condition and source IDs `sep-argument`, `open-logic-project`. The
original law above instead represents
`philosophy:explanation_phenomenon ->
philosophy:explanation_explanatory_hypothesis`, with different condition keys
and evidence/provenance IDs. Their directions and meanings are not equivalent.
No typed join is currently licensed.
