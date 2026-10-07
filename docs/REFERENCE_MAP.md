# Reference and port boundary

The source projects are read-only references. This workspace contains a new
implementation; similar type or lane names do not establish behavioral parity.

| Original artifact | SHA-256 | Relevant behavior |
| --- | --- | --- |
| `/home/jesse/Desktop/Projects/atom-harness/atom_cognitive_engine.py` | `374876a07b98ae77d1bac42e78301ddce40d2c8810a28c174b1835382a767b4c` | Hash-bound world registry, strategy plan, cross-world paths independent of selected direct routes, signed/conflict resolution, Mother context, outcome ledger. |
| `/home/jesse/Desktop/Projects/atom-harness/atom_causal_graph.py` | `c68f51cb4b2cebde306b6aa41b6d54f4f939b3901c135ba0b0a98571f430d685` | Typed causal laws, observations, conditions, graph composition and bounded queries. |
| `/home/jesse/Desktop/Projects/atom-harness/atom_philosophy_world.py` | `90e957d7066fc33c130119a14f02de582845a77837cf39a283b054191bd70c9c` | Source-bound signed argument relations and the reasoning-family feature shapes. |
| `/home/jesse/Desktop/Projects/atom-harness/atom_creativity_engine.py` | `6cc2a04d5a988d68b6f23530268e3723c96879fe5aa26de547f655615fb17e39` | Seven creative mechanisms with hard constraints, blending, framing, tension bridges, novelty/value, memory material, and selection. |
| `/home/jesse/Desktop/Projects/atom-harness/atom_integrated_creativity_engine.py` | `22af0de70e60cd79a379c87c1f6b8e3cb7b302d8eb6f773d53bf8e032b2f0901` | Coupled grounding, focus, creative generation, 16 cognitive operations, and proposal-only probability response. |

The original Python files are historical behavioral references. Atom cognition
in this project is Rust; Python is not used as the cognitive engine. The
current Rust seven-lane implementation is an initial typed mechanism set, not
a line-for-line port. In particular, it lacks the original's stateful trace
engine and Mother-bound profile. Its probability output is uncalibrated and
never promoted to factual confidence.

The active local model candidate is the LM Studio GGUF at
`/home/jesse/.lmstudio/models/OS-Software/Ternary-Bonsai-2-27B-Uncensored-Heretic-GGUF/Ternary-Bonsai-2-27B-Uncensored-Heretic-PQ2_0.gguf`.
SHA-256: `c0959544f1422c569d07ccef8911dc0d016034bc1285d91010a6630fdf4c4dda`.
The recovered harness model copy has the same measured digest; this does not
prove either model has been used by the new runtime.
