# Evaluation status

Current status: **component and one real Lightcone boundary tested; no live
model comparison**. The development and held-out corpora, scoring, and pass
gates for this new standalone harness have not been frozen.

A local model diagnostic used the pinned LM Studio Bonsai-2 GGUF with the
existing llama.cpp GPU build `b1-f40a7bf` on an RTX 5070 Ti. A bounded
`llama-completion` run loaded the model and generated 16 tokens at reported
73.94 tokens/s decode, but remained in conversation mode and exhausted its
budget in thinking text. A preceding `llama-cli` probe stayed interactive on
closed stdin and flooded output; it was terminated. Neither run produced a
scoreable answer or touched the new Atom runtime. See the retained diagnostic
record below.

Before any effectiveness claim, freeze exact prompt tasks, source classes,
unknowns, counterexamples, model hash, inference library, chat template,
sampler, seed, context budget, bare and Atom arms, scoring, correctness gates,
and protected regressions. Keep the only experimental difference explicit.
Score raw model generations separately from host-released outputs; do not count
host replacement, constraint text, or source hints as independent model gains.

Required boundaries for the eventual matched run:

1. Every selected reasoning lane executes its own typed contract and returns
   provenance. A name in a prompt is not execution.
2. The seven creative mechanisms run with actual reasoning, probability and
   prediction consumers; component-only tests remain diagnostics.
3. Real passive Lightcone admissions bind the exact pack and receipt at the
   model's sampling boundary. Unknown and out-of-cone controls stay unknown.
4. Local Bonsai generation uses the pinned GGUF and exact inference build.
   A copied model file or a verified pack alone is not live inference.
5. Held-out accuracy, failure modes, token/latency cost, and ablations are
   compared on identical frozen inputs. Failed runs remain in evidence.

Executive status, 2026-09-24: the executive is in the loop (rev 5 of the protocol).
Selection, bridges, the fast/slow dual process with conflict failing closed, and the
never-empty creative guarantee are implemented and pinned by contract tests; the runner
dispatches through them and the nine-case development corpus passes. This satisfies the
consumer half of requirement 2 in the operator's design — the creative lane answers when
reasoning cannot, with the contracted probability response.

Boundary status, 2026-09-24: requirement 3 (sampling-boundary binding) and the
open half of requirement 2 (creative, probability and prediction consumers) are
implemented and diagnosed on the development corpus. The membrane now has four
lanes in one process — input admission, the post-weights Lightcone sampler, the
engine-side output check and release decision, and verified-claim feedback into
a bounded persistent warrant ledger — with the seven creative mechanisms
dispatching in the runner and their consumers reported separately. See the
protocol's revision-3 and revision-4 sections. Requirement 5 remains open: the
held-out corpus, correctness scoring, protected regressions, and the
original-mechanism parity gates are the operator's to freeze, and no
effectiveness claim follows from a boundary or wiring test.

No Ordo, Ordo tool-loop, Ordo TPM, or Ordo acceptance gate is part of this
standalone project's protocol.
