# Proposed Lightcone-to-CausalLaw warrants — review required

Status: **draft only; not installed or consumed by any runtime**. This is a
proposal in response to the user's request to review cross-ontology warrants.
It does not equate node IDs, transfer confidence, or make Lightcone a factual
judge. Each proposal would require a machine-checked scope, exact pack/model
hashes, source references and negative controls before activation.

## Candidate W1: hypothesis role alignment

- Lightcone source: `scientific-reasoning-v0/0.2.0`, node
  `science-reasoning.v0.hypothesis`; its statement defines a proposed
  explanation or relationship with testable consequences and defeaters.
  Admission receipt for `alternative hypothesis`:
  `59e8b4004a436a6bea9dae495103d934bc29db1de25d8e9cea7197dfeec749e9`.
- Sourced-law target: `philosophy-reasoning-foundation-v1`, feature
  `philosophy:explanation_explanatory_hypothesis`, visible as the effect of
  `law:01e07c02cafea26c28d59aac`; model hash
  `f0f3d739c8533ee801828e573791acfb1ee0143e91d71c19bab41c276140b2b8`.
- Proposed typed relation: `qualifies` — the Lightcone node may supply a
  *scientific-hypothesis role constraint* when interpreting that law feature
  in a scientific-explanation task. It does **not** assert feature identity,
  cause/effect direction, law confidence, or observational truth.
- Source lead: the Lightcone node cites `sep-abduction` among its source IDs;
  the original philosophy explanation branch declares `sep-abduction` and
  `sep-scientific-method`. This is a source-overlap lead, not yet a checked
  claim-level correspondence.
- Activation condition: user approves the narrow scope and an independent
  source check confirms both records use `hypothesis` in this role. Otherwise
  the warrant must remain absent.

## Candidate W2: explanation evaluation context

- Lightcone source: `scientific-reasoning-v0/0.2.0`, node
  `science-reasoning.v0.explanation`, receipt
  `9fe7f5a34c5256593bcbe32f5909330d3df51dc01b8cc1a056ee174fe2b866c5`.
  Its statement requires a declared explanatory standard and allows laws,
  mechanisms, models, causes, or unifying structure.
- Sourced-law target: the *explanation branch* of the copied philosophy world,
  not a specific CausalLaw edge or a renamed Lightcone node. The branch
  contains separate `phenomenon`, `explanatory_hypothesis`, `scope_and_unification`,
  `explanatory_power`, and `understanding` features.
- Proposed typed relation: `qualifies` — the Lightcone definition could
  constrain the interpretation of an explanation result's scope and
  alternatives. It may not change a law direction, synthesize missing support,
  or substitute for the branch's own evidence.
- Activation condition: review the exact branch curriculum and the Lightcone
  source passages; approve only a branch-level interpretive warrant, never
  automatic feature equivalence.

## Rejected automatic mapping R1

Do **not** map Lightcone `p.hypothesis -> p.explanation` from
`philosophy-rhetoric-v0/0.2.0` directly onto original
`philosophy:explanation_phenomenon ->
philosophy:explanation_explanatory_hypothesis`. The directions, conditions,
and source IDs differ. The Lightcone edge has condition “the reconstructed
context and meanings are held fixed” and interpretive node classes; the
original law has `inquiry:general` and
`philosophy_branch:explanation` contexts and distinct evidence IDs. Similar
words are insufficient for a typed causal or factual join.

## Review decision

Please mark W1 and W2 approved, rejected, or revised. Approval would
authorize only construction and testing of the specified narrow warrants;
the runtime must still fail closed on pack/model hash drift, missing sources,
out-of-scope tasks, conflicting direction, and absent evidence. Until then,
`lightcone_sourced_law_typed_join=false` remains the live behavior.
