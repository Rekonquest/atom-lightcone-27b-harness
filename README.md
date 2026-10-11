# Standalone Atom–Lightcone harness

This project is being rebuilt without Ordo. It has a Rust Atom reasoning and
creative component chain, a fixed orchestrator, a passive Lightcone adapter,
and an experimental native Prism post-weights runner. It is **not yet** a
complete replacement for the original cognitive and creative harness, and no
model-performance improvement is claimed.

## Ownership

| Component | Owner and current boundary |
| --- | --- |
| `crates/atom-causal-primitives` | Typed admitted edges, validation and bounded path composition. No Lightcone receipt or factual verdict. |
| `crates/atom-causal-laws` | Typed original CausalLaws, separate from Lightcone edges; source-bound routing and within/across-world path search. |
| `crates/atom-creativity` | Seven bounded creative operations with hard constraints and traceable candidate provenance. No factual authority. |
| `crates/atom-reasoning-engine` | Initial execution of seven distinct reasoning lanes and a typed cognitive-to-creative call. It is not original executive parity. |
| `crates/atom-executive` | The policy layer: deterministic multi-lane selection with a reason per lane, one dispatch per lane contract, authoritative aggregation where cross-lane conflict fails closed to unknown, bridges carrying provenance and evidence class only, and the never-empty guarantee (creative paths, or one labelled speculative fallback). No factual authority, no merged score. |
| `crates/atom-creative-paths` | The contracted creative probability response: several labelled speculative paths with independent uncalibrated feasibility estimates and uncertainty bands, no declared winner, proposals only. Not a causal-geometric lane. |
| `crates/atom-orchestrator` | Fixed `route()` → engine `dispatch()` → return, plus the engine-side output boundary (`boundary.rs`): the admitted claim is compared with what the text asserts and a release decision follows. No task-dependent routing, no model calls, and the boundary holds no factual authority (it is the Atom side acting through the boundary, not the Lightcone). |
| `lightcone/` | Local copy of the passive Lightcone code, runtime world packs, ontology, schemas, adapters, and receipts. Its Cargo workspace and external libraries remain outside the Atom crates. |
| `runtime/harness-cli` | Local process adapter. The copied Lightcone CLI verifies a pack and admits a bounded neighborhood; the adapter preserves relation, polarity, epistemic classes, domains, conditions, exceptions, and source IDs before calling Atom. It passes **no invented confidence**. |
| `runtime/native-harness` | Standalone Prism GGUF runner; the grounded arm dispatches through the executive, `--creative` supplies material rather than forcing a dispatch, and `--reasoning-brief` renders the executive's answer into the model's context. Read raw logits reading raw logits after decode. Four lanes in one process: Atom dispatches on the first post-weights slice; a registered `lightcone-direct-post-weights` sampler binds the admitted vocabulary over raw logits before token selection (exact pass-through when nothing is admitted); the engine-side output check decides what may cross; and a verified claim reinforces a bounded persistent warrant ledger. `--creative <brief.json>` dispatches the seven creative mechanisms; `src/bin/corpus.rs` runs the development corpus gates. The rejected path-count temperature response is disabled by default and available only as an explicit diagnostic experiment. |
| `reference-causal-worlds` | User-authorized byte copies of five original world packs and one archival base model, with pinned hashes. The archived base model fails the original loader and is not active. |

The Atom Cargo workspace has no external dependencies; its only dependencies
are one-way edges among in-repository Atom crates. The host adapters are
outside Atom and use `serde_json` to decode the separate Lightcone process
output. No Ordo code or service is involved.

## Direct model boundary

The enforced order on the local 27B is the old harness's contract:

```text
Prism weights -> raw logits -> lightcone-direct-post-weights -> token selection
```

A verified passive admission exposes admitted nodes; their labels and aliases
become a bounded, adapter-owned set of preferred tokens, and a registered
sampler named `lightcone-direct-post-weights` applies them over the raw logits
before this harness's own selection stage. It fires only on an undecided top-2
margin and adds at most `bias_cap`. An unknown or out-of-cone query, or a pack
that admitted nothing, is exact pass-through: not one logit changes. Lightcone
stays passive geometry — no verdict, no score, no authorization, no tool
permission — and every bound is an adapter constant. The rejected path-count
temperature operation is a separate mechanism, disabled by default, and it keeps
its no-gain evidence.

See the [native diagnostic protocol](docs/NATIVE-DIAGNOSTIC-PROTOCOL.md) for the
revision-3 runs and their receipts.

## Reproduce the current checks

```bash
cargo test --offline
cargo build --offline -p lightcone-cli --manifest-path lightcone/Cargo.toml
cargo test --offline --manifest-path runtime/harness-cli/Cargo.toml
cargo test --offline --manifest-path runtime/native-harness/Cargo.toml
runtime/harness-cli/target/debug/harness-cli \
  lightcone/artifacts/worlds/network-security-browser-v0/0.2.0 \
  'TCP acknowledgment' net.tcp-ack net.retransmission prove
```

The four-lane membrane, gated per case by the development corpus:

```bash
cargo build --offline --manifest-path runtime/native-harness/Cargo.toml
runtime/native-harness/target/debug/corpus \
  /home/jesse/.lmstudio/models/OS-Software/Ternary-Bonsai-2-27B-Uncensored-Heretic-GGUF/Ternary-Bonsai-2-27B-Uncensored-Heretic-PQ2_0.gguf \
  lightcone/artifacts/worlds/network-security-browser-v0/0.2.0 \
  runtime/native-harness/fixtures/development-corpus.json \
  --creative runtime/native-harness/fixtures/creative-brief-development.json \
  --brief
```

With `--brief`, the atom arm's context carries the admitted geometry — every
admitted typed claim with labels, relation kind, class, conditions and source
ids, then the lanes artifact. The bare arm never sees it. Under that
configuration the correctness gate reads bare 1/2, atom 2/2 (revision 7 of the
diagnostic protocol), and the wiring corpus passes 9/9, reproduced
byte-identically.

The sealed-pack example returns a passive receipt and the admitted
`nsb.relation.net-ack-retransmit` enabling edge (established class). Its five
source IDs and its declared condition remain visible. The Atom path has
`strength: None` — the pack declares provenance, not calibration — and the
output explicitly has no factual-claim authority.

The placeholder pack these examples used before revision 6
(`creativity-thinking-v0`, 100 nodes) was removed on 2026-09-24; the harness's
world store is now exactly the sealed cones.

The causal goal on the same input also returns `Unknown`, and the reason is in
the code rather than in the relation kind: every `Link` the adapter builds
carries `confidence: None`, both the causal and probabilistic lanes fold over
`path.strength`, and an empty fold cannot produce a position. Until the mapper
carries calibration, those two lanes report `Unknown` over admitted `causes`
edges while the deduction lane answers structurally. Unknown seed controls admit
no path.
Tests use synthetic inputs only to exercise algorithms, not to establish
real-world facts or measured model uplift.

## Current limits

- The approved one-way crate split, including the additional causal-laws crate,
  is complete. Each normally formatted Atom crate is below 500 physical lines.
  The old compacted engine was removed.
- The Rust executive implements seven distinct initial lane contracts, but it
  does not reproduce the original Mother-bound executive, resident learning,
  full world routing/resolution, or original benchmark performance. The new
  sourced-law executor handles direct and cross-world paths and signed conflict
  in initial form, but is not original executive parity.
- The seven creative operations now compose with reasoning in a typed Rust API,
  and they dispatch inside the native runner under `--creative <brief.json>`,
  with per-lane evidence weights, the prediction relation, and contested-evidence
  status consumed and reported separately. The original integrated engine's
  full cognitive-operation set and its proposal-only probability response are
  still not reproduced, and no calibration of any weight is claimed.
- The local 27B model runs through a real post-weights/before-selection Atom
  dispatch. The currently tested path-count temperature mechanism changed
  tokens but did not show a quality gain in three development cases. It is not
  accepted as the final cognitive intervention; see the
  [native diagnostic protocol](docs/NATIVE-DIAGNOSTIC-PROTOCOL.md).
- The passive Lightcone now binds at the sampling boundary: admitted vocabulary
  becomes bounded token preferences over raw logits, with exact pass-through
  when nothing is admitted. That is a wiring and boundary result, not a
  model-performance result — whether it improves an answer is not measured, and
  the held-out comparison is still unfrozen.
- The copied Lightcone source HEAD differs from the source revision named in
  its catalog-v2 seal. The pack used above verified under the copied runtime;
  that does not create a new accelerator acceptance for this source revision.
- Model-effectiveness corpus and matched bare-versus-Atom protocol are not
  frozen. No effectiveness claim is licensed by component tests or one pack.

See [reference map](docs/REFERENCE_MAP.md), [evaluation status](docs/EVALUATION.md),
and [copied Lightcone provenance](lightcone/INTEGRATION.md).

## License

Licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE) (`PolyForm-Noncommercial-1.0.0`). Free for personal use, research, education, charities, public-good nonprofits, and government; commercial use is not permitted. This is source-available software, not OSI open source.

Third-party and derived components in this repository keep their original licenses and are **not** covered by the PolyForm license:

- cap-* crates extracted from or modeled on Zed (upstream Apache-2.0 / GPL-3.0 portions keep their original licenses)
- llama.cpp / ggml (MIT, Copyright The ggml authors)
- vendored third-party crates/libraries (their own licenses)
