//! The executive: policy over lanes, and nothing else.
//!
//! See `docs/EXECUTIVE.md` for the contract this implements. The shape comes
//! from the operator's own artifacts: lanes are mechanisms with their own typed
//! contracts; the executive is policy ("the orchestrator is all policy and no
//! mechanism"); and bridges hand a candidate between lanes **without merging**
//! their stages, controls, evidence classes or scores.
//!
//! Three rules govern every path through this crate:
//!
//! 1. **No factual authority.** `factual_claim_authority` is false on every
//!    outcome, and a creative candidate never claims causal truth.
//! 2. **No merged score.** Lane status, direction, evidence weight, bridge
//!    provenance and creative feasibility are reported side by side.
//! 3. **Never empty.** If no reasoning lane can serve the question, the creative
//!    lane answers with several labelled speculative paths carrying
//!    uncalibrated feasibility estimates and no declared winner; if it has no
//!    material either, one labelled speculative fallback is emitted. The
//!    harness never fails to produce an answer or task.

use atom_orchestrator::{
    CreativeInput, Engine, Error, Goal, Input, Lane, LaneResult, Question, Relation, Sign,
    Status, StackRule,
};
pub use atom_creative_paths::CreativePath;

/// The rung that produced an outcome. Reported on every result so a caller can
/// see which layer answered, not just what it said.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rung {
    /// The fastest selected lane supplied a provisional proposal.
    Fast,
    /// Every selected lane ran and the result is authoritative.
    Slow,
    /// No reasoning lane served the question; the creative lane answered.
    Creative,
    /// Neither reasoning nor creativity had material; a labelled speculative
    /// fallback stands in, so the result is never empty.
    SpeculativeFallback,
}

/// Why a lane joined the selection. Policy is auditable, so each decision
/// carries its typed reason.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Selection {
    pub lanes: Vec<Lane>,
    pub reasons: Vec<(&'static str, &'static str)>,
}

/// A hand-off between lanes. It carries provenance and the candidate's evidence
/// class — and deliberately carries **no** status, direction, or weight from the
/// lane that produced it.
#[derive(Clone, Debug)]
pub struct Bridge {
    pub from: Lane,
    pub to: Lane,
    pub relation: Relation,
    pub link_ids: Vec<String>,
    pub source_ids: Vec<String>,
    pub evidence_class: atom_orchestrator::EvidenceClass,
}

/// One lane's own position, reported as its own quantity.
#[derive(Clone, Debug)]
pub struct LanePosition {
    pub lane: Lane,
    pub status: Status,
    pub direction: Option<Sign>,
    pub evidence_weight: Option<f64>,
    pub paths: usize,
    pub hypotheses: usize,
    pub reason: &'static str,
}

/// The executive's answer to one question.
#[derive(Clone, Debug)]
pub struct Outcome {
    pub rung: Rung,
    pub selection: Selection,
    /// Authoritative position: the slow pass where it ran, the fast proposal
    /// otherwise.
    pub status: Status,
    pub direction: Option<Sign>,
    pub positions: Vec<LanePosition>,
    /// The raw per-lane contract output, kept so a caller can consume each
    /// lane's own paths and hypotheses without the executive re-deriving them.
    /// Reported beside everything else; never merged.
    pub lanes: Vec<LaneResult>,
    pub bridges: Vec<Bridge>,
    pub creative: Vec<CreativePath>,
    /// Unresolved signed conflict. Conflict is never averaged away; it is
    /// reported and the status fails closed to unknown.
    pub conflict: bool,
    /// Always false.
    pub factual_claim_authority: bool,
    /// Always false: no winner is declared among creative paths.
    pub winner_declared: bool,
}

/// One lane's own goal. The engine plans lanes from a goal, so the executive
/// drives one dispatch per lane contract rather than reaching into the engine.
const LANE_GOALS: &[(Lane, Goal)] = &[
    (Lane::Deduction, Goal::Prove),
    (Lane::Induction, Goal::Generalize),
    (Lane::Abduction, Goal::Explain),
    (Lane::Analogy, Goal::Transfer),
    (Lane::Causal, Goal::TraceCause),
    (Lane::Counterfactual, Goal::AlterPremise),
    (Lane::Probabilistic, Goal::Estimate),
];

pub struct Executive;

impl Executive {
    /// Selection policy: deterministic, typed, and never an argmax-only choice.
    ///
    /// The ladder floor comes from the goal, then typed features of the question
    /// and the admission add lanes. Every addition records why.
    pub fn select(input: &Input) -> Selection {
        let mut lanes: Vec<Lane> = Vec::new();
        let mut reasons: Vec<(&'static str, &'static str)> = Vec::new();
        let add = |lane: Lane, reason: &'static str, reasons: &mut Vec<(&'static str, &'static str)>| {
            if !reasons.iter().any(|(existing, _)| *existing == lane_name(lane)) {
                reasons.push((lane_name(lane), reason));
            }
        };
        let question = &input.question;
        let links = &input.admission.links;

        // The goal's own lane is always present: the engine plans it, and the
        // executive never second-guesses the operator's question.
        for (lane, goal) in LANE_GOALS {
            if *goal == question.goal {
                add(*lane, "the question's goal plans this lane", &mut reasons);
                if *lane == Lane::Causal {
                    add(Lane::Deduction, "causal tracing plans deduction beside it", &mut reasons);
                }
            }
        }
        if question.analogy.is_some() {
            add(Lane::Analogy, "an explicit analogy mapping is present", &mut reasons);
        }
        if question.removed_link.is_some() {
            add(Lane::Counterfactual, "a link to remove is declared", &mut reasons);
        }
        if links.iter().any(|link| link.relation == Relation::Predicts) {
            add(Lane::Induction, "the admission carries a prediction edge", &mut reasons);
        }
        let worlds: std::collections::BTreeSet<&str> =
            links.iter().map(|link| link.world.as_str()).collect();
        if worlds.len() >= 2 {
            add(Lane::Induction, "the admission spans independent worlds", &mut reasons);
        }
        let effects = links
            .iter()
            .filter(|link| link.effect == question.effect)
            .map(|link| link.cause.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        if effects.len() >= 2 {
            add(Lane::Abduction, "several admitted causes reach the effect", &mut reasons);
        }
        if links.iter().any(|link| {
            link.sign == Sign::Opposes
                && question.cause.as_deref().is_none_or(|cause| link.cause == cause)
        }) {
            add(Lane::Probabilistic, "admitted evidence opposes the question's direction", &mut reasons);
        }
        if links.iter().any(|link| {
            link.effect == question.effect
                && matches!(
                    link.class,
                    atom_orchestrator::EvidenceClass::Established
                        | atom_orchestrator::EvidenceClass::Formal
                )
                && matches!(
                    link.target_class,
                    atom_orchestrator::EvidenceClass::Established
                        | atom_orchestrator::EvidenceClass::Formal
                )
        }) {
            add(Lane::Deduction, "established warrant is available for the effect", &mut reasons);
        }

        for (lane, _) in LANE_GOALS {
            if reasons.iter().any(|(name, _)| *name == lane_name(*lane)) {
                lanes.push(*lane);
            }
        }
        Selection { lanes, reasons }
    }
}

fn lane_name(lane: Lane) -> &'static str {
    match lane {
        Lane::Deduction => "Deduction",
        Lane::Induction => "Induction",
        Lane::Abduction => "Abduction",
        Lane::Analogy => "Analogy",
        Lane::Causal => "Causal",
        Lane::Counterfactual => "Counterfactual",
        Lane::Probabilistic => "Probabilistic",
    }
}

/// Rank lanes by what they actually produced: a supported path first, then
/// provisional, then unknown; ties keep the selection order. This is policy over
/// reported positions, not a blended score.
fn rank(positions: &[LanePosition]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..positions.len()).collect();
    order.sort_by_key(|index| match positions[*index].status {
        Status::SupportedPath => 0,
        Status::Provisional => 1,
        Status::Unknown => 2,
    });
    order
}

fn stronger(candidate: Status, existing: Status) -> bool {
    match (candidate, existing) {
        (Status::SupportedPath, status) => status != Status::SupportedPath,
        (Status::Provisional, Status::Unknown) => true,
        _ => false,
    }
}

impl Executive {
    /// One lane's own question. The engine plans lanes from a goal, so the
    /// executive drives one dispatch per lane contract and lets each lane apply
    /// its own preconditions — a lane whose contract cannot run is skipped with
    /// its reason recorded, never forced.
    fn lane_question(input: &Input, lane: Lane) -> Result<Question, &'static str> {
        let goal = LANE_GOALS
            .iter()
            .find(|(candidate, _)| *candidate == lane)
            .map(|(_, goal)| *goal)
            .ok_or("unknown lane")?;
        let mut question = input.question.clone();
        question.goal = goal;
        if question.cause.is_none()
            && matches!(
                lane,
                Lane::Deduction | Lane::Induction | Lane::Causal | Lane::Counterfactual | Lane::Probabilistic
            )
        {
            return Err("skipped: this lane needs a cause and the question has none");
        }
        if lane == Lane::Analogy && question.analogy.is_none() {
            return Err("skipped: no analogy mapping on the question");
        }
        if lane == Lane::Counterfactual && question.removed_link.is_none() {
            return Err("skipped: no link declared for removal");
        }
        if lane == Lane::Counterfactual {
            if let Some(link) = question.removed_link.as_ref() {
                if !input.admission.links.iter().any(|admitted| &admitted.id == link) {
                    return Err("skipped: the link to remove is not in the admission");
                }
            }
        }
        Ok(question)
    }

    /// The authoritative position, with conflict never averaged away.
    fn authoritative(positions: &[LanePosition]) -> (Status, Option<Sign>, bool) {
        let directions: Vec<Sign> = positions
            .iter()
            .filter(|position| position.status == Status::SupportedPath)
            .filter_map(|position| position.direction)
            .collect();
        let conflict = directions.contains(&Sign::Supports) && directions.contains(&Sign::Opposes);
        if conflict {
            return (Status::Unknown, None, true);
        }
        if let Some(direction) = directions.first() {
            return (Status::SupportedPath, Some(*direction), false);
        }
        let provisional: Vec<Option<Sign>> = positions
            .iter()
            .filter(|position| position.status == Status::Provisional)
            .map(|position| position.direction)
            .collect();
        if provisional.is_empty() {
            return (Status::Unknown, None, false);
        }
        let mut distinct: Vec<Option<Sign>> = Vec::new();
        for direction in &provisional {
            if !distinct.contains(direction) {
                distinct.push(*direction);
            }
        }
        let direction = if distinct.len() == 1 { distinct[0] } else { None };
        (Status::Provisional, direction, false)
    }

    /// Bridges: hand a candidate between two lanes that cite the same admitted
    /// material, carrying provenance and the evidence class only. Nothing of the
    /// source lane's position travels with it — the type has no status, no
    /// direction and no weight to carry, which is how the doctrine's "do not
    /// merge their stages, controls, evidence classes, or scores" is enforced by
    /// construction rather than by discipline.
    fn bridges(input: &Input, results: &[LaneResult]) -> Vec<Bridge> {
        let mut out: Vec<Bridge> = Vec::new();
        for (index, from) in results.iter().enumerate() {
            if from.paths.is_empty() {
                continue;
            }
            for to in results.iter().skip(index + 1) {
                if to.paths.is_empty() {
                    continue;
                }
                let mut link_ids: Vec<String> = Vec::new();
                let mut source_ids: Vec<String> = Vec::new();
                for from_path in &from.paths {
                    for to_path in &to.paths {
                        for id in &from_path.link_ids {
                            if to_path.link_ids.contains(id) && !link_ids.contains(id) {
                                link_ids.push(id.clone());
                            }
                        }
                        for source in &from_path.source_ids {
                            if to_path.source_ids.contains(source) && !source_ids.contains(source) {
                                source_ids.push(source.clone());
                            }
                        }
                    }
                }
                if link_ids.is_empty() && source_ids.is_empty() {
                    continue;
                }
                let link = link_ids
                    .first()
                    .and_then(|id| input.admission.links.iter().find(|link| &link.id == id))
                    .or_else(|| {
                        input
                            .admission
                            .links
                            .iter()
                            .find(|link| link.source_ids.iter().any(|source| source_ids.contains(source)))
                    });
                let Some(link) = link else { continue };
                out.push(Bridge {
                    from: from.lane,
                    to: to.lane,
                    relation: link.relation,
                    link_ids,
                    source_ids,
                    evidence_class: link.class,
                });
            }
        }
        out
    }

    /// Run the question end to end and never return empty.
    pub fn execute(
        engine: &Engine,
        input: Input,
        brief: Option<CreativeInput>,
    ) -> Result<Outcome, Error> {
        let selection = Self::select(&input);
        let mut results: Vec<LaneResult> = Vec::new();
        let mut skipped: Vec<LanePosition> = Vec::new();
        for lane in &selection.lanes {
            let question = match Self::lane_question(&input, *lane) {
                Ok(question) => question,
                Err(reason) => {
                    skipped.push(LanePosition {
                        lane: *lane,
                        status: Status::Unknown,
                        direction: None,
                        evidence_weight: None,
                        paths: 0,
                        hypotheses: 0,
                        reason,
                    });
                    continue;
                }
            };
            let report = match atom_orchestrator::run(
                engine,
                Input {
                    admission: input.admission.clone(),
                    question,
                },
            ) {
                Ok(report) => report,
                // A lane that refuses its own input becomes a skipped lane,
                // never a missing answer. A position carries only a static
                // reason, so the specific precondition is checked above and
                // this catch is the totality guard behind it.
                Err(_) => {
                    skipped.push(LanePosition {
                        lane: *lane,
                        status: Status::Unknown,
                        direction: None,
                        evidence_weight: None,
                        paths: 0,
                        hypotheses: 0,
                        reason: "skipped: the lane refused its own input",
                    });
                    continue;
                }
            };
            for result in report.lanes {
                match results.iter_mut().find(|existing| existing.lane == result.lane) {
                    Some(existing) if stronger(result.status, existing.status) => *existing = result,
                    Some(_) => {}
                    None => results.push(result),
                }
            }
        }
        let lanes = results.clone();
        let mut positions: Vec<LanePosition> = results
            .iter()
            .map(|result| LanePosition {
                lane: result.lane,
                status: result.status,
                direction: result.direction,
                evidence_weight: result.probability,
                paths: result.paths.len(),
                hypotheses: result.hypotheses.len(),
                reason: result.reason,
            })
            .collect();
        positions.extend(skipped);

        let ranked = rank(&positions);
        let fast_status =
            ranked.first().map_or(Status::Unknown, |index| positions[*index].status);
        let (status, direction, conflict) = Self::authoritative(&positions);
        let bridges = Self::bridges(&input, &results);

        // Nothing reasoning could stand on: the creative lane answers, because
        // the harness never fails to produce an answer or task.
        if status == Status::Unknown {
            let (creative, speculative) =
                match atom_creative_paths::creative_paths(engine, &input, brief) {
                    Ok(answer) => answer,
                    // A refused brief still answers through the labelled,
                    // scored, non-factual fallback: degraded and marked, never
                    // absent.
                    Err(_) => (vec![atom_creative_paths::speculative_fallback(&input)], true),
                };
            let rung = if speculative { Rung::SpeculativeFallback } else { Rung::Creative };
            return Ok(Outcome {
                rung,
                selection,
                status,
                direction,
                positions,
                lanes,
                bridges,
                creative,
                conflict,
                factual_claim_authority: false,
                winner_declared: false,
            });
        }

        let _ = fast_status;
        Ok(Outcome {
            rung: if status == Status::SupportedPath { Rung::Slow } else { Rung::Fast },
            selection,
            status,
            direction,
            positions,
            lanes,
            bridges,
            creative: Vec::new(),
            conflict,
            factual_claim_authority: false,
            winner_declared: false,
        })
    }
}

/// The engine's fixed entrypoint, so callers cannot route around the executive.
pub fn run(engine: &Engine, input: Input, brief: Option<CreativeInput>) -> Result<Outcome, Error> {
    let _ = StackRule::EvidenceFirst;
    Executive::execute(engine, input, brief)
}
