# Cross-world source-translation check, frozen 2026-09-23

This is a bounded validation of the opt-in structured-translation experiment,
not the original harness's complete final acceptance or a general intelligence
claim. The intervention was developed on the philosophy explanation case;
the four cases below use different copied world packs and had not been run
through the candidate model when this protocol was written. The original
world files are hash-pinned in `reference-causal-worlds/MANIFEST.json`.

Common setup: LM Studio Bonsai-2 Uncensored Heretic `PQ2_0` file SHA-256
`c0959544f1422c569d07ccef8911dc0d016034bc1285d91010a6630fdf4c4dda`;
local Prism fork source commit `f40a7bf69ba1ac1c07608fd675606fa0e090efc3`;
RTX 5070 Ti; passive Lightcone pack
`creativity-thinking-v0/0.2.0` manifest hash
`876a84fe63b9193ada82015956e36a5de1e5a051cce7e5ea92936d8a0bd32aaa`
(*placeholder pack, removed 2026-09-24; the harness now runs
`network-security-browser-v0/0.2.0`, manifest SHA-256 `2588a0290eb3118206f0d8fbb427a2b025a74d8eb4ef6b55cba940606e175602`*).
Measured executable SHA-256:
`4c46092d15aca33ea551f59fe56326b540bf6f5dc282d41c85fdcd3f6bb9112c`;
local `libllama.so`:
`88f94a85e4b6c3a0f7988adfd47fd16399cc4e00377dd06427a7a7f59286d7de`;
local `libggml-cuda.so`:
`bb518dd90a1d1ae2e606aabea0686a76a6b6dbd1ae02233bfa5ee104146aa647`.
Both arms use the same query, non-thinking chat continuation, top-k 20,
top-p 0.95, temperature 1.0, seed `0x8d4942d5`, 96-token cap and fresh
context reset. Run the candidate with
`--json --no-think --experimental-structured-translation --law`.
Only the Atom arm receives a post-weights-triggered restart with the typed
source artifact; the Lightcone and sourced-law channels are not joined.

| Case | User query | Sourced-law cause → effect / domain | Exact required context keys |
| --- | --- | --- | --- |
| algebra | What are the two stored context signatures for the feasible-interval to boundary-check causal law? Answer in one sentence. | `algebra_word:inequalities_feasible_interval` → `algebra_word:inequalities_boundary_check` / `symbolic` | `algebra_word_branch:inequalities`, `problem:word-algebra` |
| coding | For the numerical error-bound to reliability law, name its exact recorded contexts in one sentence. | `coding:numerical_error_bound` → `coding:numerical_numerical_reliability` / `symbolic` | `coding_branch:numerical`, `runtime:general` |
| psychology | Which context tags belong to the feedback-quality to motor-learning law? Reply with both tags only. | `feedback_quality` → `motor_learning` / `biological` | `population:general`, `psychology_branch:sensorimotor` |
| tool | For the directory declared-path to workspace-confinement law, list the original context keys. | `tool:directory_declared_path` → `tool:directory_workspace_confinement` / `agent` | `runtime:permissioned-hands`, `tool_use_branch:directory` |

For each case, check that a real sourced-law path is supported, structured
translation was used, the released answer names both exact keys, and no
additional context key is presented as belonging to that law. Preserve the
bare raw answer and model tokens; do not replace it with a host answer. The
candidate passes this narrow check only at 4/4, with no unknown-control or
ordinary-task regression. The bare arm must be scored by the same rule.
Prompt wording, scorer and thresholds are fixed here before candidate runs.

Unknown control: repeat one case with cause `absent:feature` and effect
`absent:effect`; it must return `Unknown`, skip the restart and produce
byte-identical bare/Atom text. Ordinary control: query `What is 2 + 2?` with
no sourced law and no admitted Lightcone path; both arms must return the same
answer. These controls are not evidence of uplift; they protect against
unwanted behavior. Passing all six cases would validate this source-specific
translation boundary only. A separate truly unseen mechanism-level final set
and original-executive/creative parity remain necessary before any broad
effectiveness or completion claim.
