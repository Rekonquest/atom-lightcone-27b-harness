//! Typed reasoning execution over admitted causal paths.

pub use atom_creativity::*;
use std::collections::{BTreeMap, BTreeSet};

pub struct Engine;

#[derive(Clone, Debug)]
pub struct CreativeInput {
    pub reasoning: Input,
    pub brief: Brief,
    pub concepts: Vec<Concept>,
    pub config: CreativeConfig,
}

#[derive(Clone, Debug)]
pub struct CognitiveCreativeReport {
    pub reasoning: Report,
    pub creativity: CreativeResult,
}

#[derive(Clone, Debug)]
pub struct SourcedLawReport {
    pub plan: LawPlan,
    pub considered_paths: Vec<LawPath>,
    pub status: Status,
    pub direction: Option<Sign>,
    pub confidence: Option<f64>,
    pub conflict: bool,
    pub factual_claim_authority: bool,
}

impl Engine {
    pub fn dispatch_laws(
        &self,
        rule: StackRule,
        registry: &LawRegistry,
        problem: &LawProblem,
    ) -> Result<SourcedLawReport, LawError> {
        let _ = match rule {
            StackRule::EvidenceFirst => rule,
        };
        let plan = registry.plan(problem)?;
        let paths = registry.paths(problem)?;
        let selected: BTreeSet<&str> = plan
            .selected
            .iter()
            .map(|route| route.world_id.as_str())
            .collect();
        let considered: Vec<LawPath> = paths
            .into_iter()
            .filter(|path| {
                path.cross_world
                    || path
                        .world_ids
                        .iter()
                        .all(|id| selected.contains(id.as_str()))
            })
            .collect();
        let mut grouped: BTreeMap<(String, i8), (f64, f64)> = BTreeMap::new();
        for path in &considered {
            let group = if path.cross_world {
                "cross-world-composition".to_string()
            } else {
                path.world_ids.first().cloned().unwrap_or_default()
            };
            let route_score = if path.cross_world {
                path.strength
            } else {
                plan.selected
                    .iter()
                    .find(|route| route.world_id == group)
                    .map(|route| route.route_score)
                    .unwrap_or(0.0)
            };
            let reliability = path
                .world_ids
                .iter()
                .filter_map(|id| {
                    registry
                        .worlds()
                        .iter()
                        .find(|world| world.id == *id)
                        .map(|world| world.reliability)
                })
                .fold(1.0_f64, f64::min);
            let weight = reliability * (0.5 + 0.5 * route_score) * path.strength;
            let entry = grouped.entry((group, path.direction)).or_insert((0.0, 0.0));
            if weight > entry.0 {
                *entry = (weight, path.strength);
            }
        }
        let totals = [-1_i8, 1_i8].map(|direction| {
            grouped
                .iter()
                .filter(|((_, sign), _)| *sign == direction)
                .map(|(_, (weight, _))| *weight)
                .sum::<f64>()
        });
        let conflict = totals[0] > 0.0 && totals[1] > 0.0;
        let (winner, leading, runner) = if totals[1] >= totals[0] {
            (Sign::Supports, totals[1], totals[0])
        } else {
            (Sign::Opposes, totals[0], totals[1])
        };
        let margin = (leading - runner) / (leading + runner).max(1e-12);
        let supported = leading >= 0.12 && (!conflict || margin >= 0.20);
        let winning_direction = if winner == Sign::Supports { 1 } else { -1 };
        let evidence = grouped
            .iter()
            .filter(|((_, direction), _)| *direction == winning_direction)
            .map(|(_, (weight, strength))| (weight * strength, weight))
            .fold((0.0, 0.0), |(a, b), (x, y)| (a + x, b + y));
        let confidence = if supported && evidence.1 > 0.0 {
            Some((leading / (leading + runner).max(1e-12)) * evidence.0 / evidence.1)
        } else {
            None
        };
        Ok(SourcedLawReport {
            plan,
            considered_paths: considered,
            status: if supported {
                Status::SupportedPath
            } else {
                Status::Unknown
            },
            direction: supported.then_some(winner),
            confidence,
            conflict,
            factual_claim_authority: false,
        })
    }

    pub fn dispatch_creative(
        &self,
        rule: StackRule,
        input: CreativeInput,
    ) -> Result<CognitiveCreativeReport, Error> {
        let reasoning = self.dispatch(rule, input.reasoning)?;
        let grounding: Vec<PathTrace> = reasoning
            .lanes
            .iter()
            .flat_map(|lane| lane.paths.iter().cloned())
            .collect();
        let creativity =
            atom_creativity::explore(&input.brief, &input.concepts, &grounding, &input.config)
                .map_err(|error| Error::Creative(format!("{error:?}")))?;
        Ok(CognitiveCreativeReport {
            reasoning,
            creativity,
        })
    }

    pub fn dispatch(&self, rule: StackRule, input: Input) -> Result<Report, Error> {
        input.admission.validate()?;
        input.question.validate()?;
        let lanes = match rule {
            StackRule::EvidenceFirst => self.plan(&input.question)?,
        };
        let eligible: Vec<Link> = input
            .admission
            .links
            .iter()
            .filter(|link| {
                input.question.allowed_domains.is_empty()
                    || (input.question.allowed_domains.contains(&link.domain)
                        && input.question.allowed_domains.contains(&link.target_domain))
            })
            .filter(|link| {
                input
                    .question
                    .conditions
                    .iter()
                    .all(|c| link.conditions.contains(c))
            })
            .filter(|link| {
                !input
                    .question
                    .conditions
                    .iter()
                    .any(|c| link.exceptions.contains(c))
            })
            .map(|link| (link.id.clone(), link.clone()))
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect();
        let results = lanes
            .iter()
            .map(|lane| self.execute(*lane, &input.question, &eligible))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Report {
            receipt_id: input.admission.receipt_id,
            question_id: input.question.id,
            lanes: results,
            factual_claim_authority: false,
        })
    }

    fn plan(&self, q: &Question) -> Result<Vec<Lane>, Error> {
        Ok(match q.goal {
            Goal::Prove => vec![Lane::Deduction],
            Goal::Generalize => vec![Lane::Induction],
            Goal::Explain => vec![Lane::Abduction],
            Goal::Transfer => vec![Lane::Analogy],
            Goal::TraceCause => vec![Lane::Causal, Lane::Deduction],
            Goal::AlterPremise => vec![Lane::Counterfactual],
            Goal::Estimate => vec![Lane::Probabilistic],
        })
    }

    fn execute(&self, lane: Lane, q: &Question, edges: &[Link]) -> Result<LaneResult, Error> {
        let relevant: Vec<Link> = edges
            .iter()
            .filter(|edge| match lane {
                Lane::Causal | Lane::Counterfactual | Lane::Abduction => matches!(
                    edge.relation,
                    Relation::Causes | Relation::Enables | Relation::Inhibits | Relation::Prevents
                ),
                Lane::Deduction => matches!(
                    edge.relation,
                    Relation::Causes
                        | Relation::Enables
                        | Relation::Supports
                        | Relation::Grounds
                        | Relation::Prerequisite
                ),
                Lane::Induction | Lane::Probabilistic => matches!(
                    edge.relation,
                    Relation::Causes
                        | Relation::Supports
                        | Relation::Predicts
                        | Relation::Contradicts
                        | Relation::Inhibits
                ),
                Lane::Analogy => true,
            })
            .cloned()
            .collect();
        let paths = atom_creativity::paths(q, &relevant)?;
        Ok(match lane {
            Lane::Deduction => {
                let established: Vec<_> = paths.iter().filter(|p| p.established).cloned().collect();
                let positive = established.iter().any(|p| p.sign == Sign::Supports);
                let negative = established.iter().any(|p| p.sign == Sign::Opposes);
                let direction = match (positive, negative) {
                    (true, false) => Some(Sign::Supports),
                    (false, true) => Some(Sign::Opposes),
                    _ => None,
                };
                LaneResult {
                    lane,
                    status: if direction.is_some() {
                        Status::SupportedPath
                    } else {
                        Status::Unknown
                    },
                    direction,
                    paths,
                    hypotheses: vec![],
                    probability: None,
                    reason: "typed path only; unknown without established warrant",
                }
            }
            Lane::Induction => {
                let direct: Vec<_> = paths
                    .into_iter()
                    .filter(|p| p.link_ids.len() == 1)
                    .collect();
                let independent: BTreeSet<_> =
                    direct.iter().filter_map(|p| p.world_ids.first()).collect();
                LaneResult {
                    lane,
                    status: if independent.len() >= 2 {
                        Status::Provisional
                    } else {
                        Status::Unknown
                    },
                    direction: None,
                    paths: direct,
                    hypotheses: vec![],
                    probability: None,
                    reason: "independent-source pattern; no universal claim",
                }
            }
            Lane::Abduction => {
                let rivals: Vec<_> = relevant
                    .iter()
                    .filter(|edge| edge.effect == q.effect)
                    .map(|edge| edge.cause.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect();
                LaneResult {
                    lane,
                    status: if rivals.is_empty() {
                        Status::Unknown
                    } else {
                        Status::Provisional
                    },
                    direction: None,
                    paths: vec![],
                    hypotheses: rivals,
                    probability: None,
                    reason: "candidate explanations require rival comparison",
                }
            }
            Lane::Analogy => {
                let mapped = q
                    .analogy
                    .as_ref()
                    .map(|m| {
                        relevant
                            .iter()
                            .filter(|edge| {
                                edge.cause == m.source_cause && edge.effect == m.source_effect
                            })
                            .count()
                    })
                    .map_or(0, |count| count);
                LaneResult {
                    lane,
                    status: if mapped > 0 {
                        Status::Provisional
                    } else {
                        Status::Unknown
                    },
                    direction: None,
                    paths: vec![],
                    hypotheses: q
                        .analogy
                        .as_ref()
                        .filter(|_| mapped > 0)
                        .map(|m| vec![format!("{} -> {}", m.target_cause, m.target_effect)])
                        .map_or_else(Vec::new, |items| items),
                    probability: None,
                    reason: "structural projection only; target claim unverified",
                }
            }
            Lane::Causal => self.causal(lane, paths)?,
            Lane::Counterfactual => {
                let removed = q
                    .removed_link
                    .as_deref()
                    .ok_or(Error::Invalid("missing removed link"))?;
                if !relevant.iter().any(|edge| edge.id == removed) {
                    return Err(Error::Invalid("counterfactual link absent from admission"));
                }
                let alternate = atom_creativity::paths(
                    q,
                    &relevant
                        .iter()
                        .filter(|edge| edge.id != removed)
                        .cloned()
                        .collect::<Vec<_>>(),
                )?;
                let changed = paths
                    .iter()
                    .any(|p| !alternate.iter().any(|other| other.link_ids == p.link_ids));
                LaneResult {
                    lane,
                    status: if changed {
                        Status::Provisional
                    } else {
                        Status::Unknown
                    },
                    direction: None,
                    paths: alternate,
                    hypotheses: vec![],
                    probability: None,
                    reason: "bounded link-removal intervention; not an observed world",
                }
            }
            Lane::Probabilistic => {
                let independent = paths.iter().enumerate().filter(|(index, p)| {
                    !paths[..*index]
                        .iter()
                        .any(|prior| prior.sign == p.sign && prior.source_ids == p.source_ids)
                });
                let (yes, no) = independent
                    .filter_map(|(_, p)| p.strength.map(|s| (p.sign, s)))
                    .fold((0.0, 0.0), |(a, b), (sign, strength)| match sign {
                        Sign::Supports => (a + strength, b),
                        Sign::Opposes => (a, b + strength),
                        Sign::Neutral => (a, b),
                    });
                let total = yes + no;
                let contested = yes > 0.0 && no > 0.0;
                LaneResult {
                    lane,
                    status: if contested {
                        Status::Provisional
                    } else {
                        Status::Unknown
                    },
                    direction: None,
                    paths,
                    hypotheses: vec![],
                    probability: if contested { Some(yes / total) } else { None },
                    reason: "contested uncalibrated evidence weight; not factual confidence",
                }
            }
        })
    }

    fn causal(&self, lane: Lane, paths: Vec<PathTrace>) -> Result<LaneResult, Error> {
        let established: Vec<&PathTrace> = paths.iter().filter(|p| p.established).collect();
        let calibrated: Vec<(Sign, f64)> = established
            .iter()
            .filter_map(|p| p.strength.map(|s| (p.sign, s)))
            .collect();
        // A pack that declares no calibration declares none. The direction a
        // cone carries in its own polarity is not a magnitude, so an
        // established path with no strength still reports its declared
        // direction — provisional, weightless — instead of vanishing into the
        // empty fold. Signed conflict among established paths still fails
        // closed.
        if calibrated.is_empty() {
            if let Some(direction) = established.first().map(|p| p.sign) {
                let conflict = established.iter().any(|p| p.sign != direction);
                return Ok(LaneResult {
                    lane,
                    status: if conflict { Status::Unknown } else { Status::Provisional },
                    direction: (!conflict).then_some(direction),
                    paths,
                    hypotheses: vec![],
                    probability: None,
                    reason: if conflict {
                        "signed conflict on established paths; no calibration declared"
                    } else {
                        "declared direction without declared calibration"
                    },
                });
            }
            return Ok(LaneResult {
                lane,
                status: Status::Unknown,
                direction: None,
                paths,
                hypotheses: vec![],
                probability: None,
                reason: "no established path",
            });
        }
        let strongest = calibrated.iter().fold(
            (0.0_f64, 0.0_f64),
            |(yes, no), (sign, strength)| match sign {
                Sign::Supports => (yes.max(*strength), no),
                Sign::Opposes => (yes, no.max(*strength)),
                Sign::Neutral => (yes, no),
            },
        );
        let total = strongest.0 + strongest.1;
        let margin = if total > 0.0 {
            (strongest.0 - strongest.1).abs() / total
        } else {
            0.0
        };
        let supported = strongest.0.max(strongest.1) >= 0.12
            && (strongest.0 == 0.0 || strongest.1 == 0.0 || margin >= 0.20);
        let direction = if supported && strongest.0 > strongest.1 {
            Some(Sign::Supports)
        } else if supported && strongest.1 > strongest.0 {
            Some(Sign::Opposes)
        } else {
            None
        };
        Ok(LaneResult {
            lane,
            status: if direction.is_some() {
                Status::SupportedPath
            } else {
                Status::Unknown
            },
            direction,
            paths,
            hypotheses: vec![],
            probability: None,
            reason: if supported {
                "bounded directed path with conflict check"
            } else {
                "no supported path or unresolved signed conflict"
            },
        })
    }
}
