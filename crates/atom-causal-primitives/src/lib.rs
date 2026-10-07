//! Typed causal execution after passive Lightcone admission. No factual verdict.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Sign {
    Supports,
    Opposes,
    Neutral,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceClass {
    Formal,
    Established,
    Provisional,
    Hypothesis,
    Contested,
    Interpretive,
    Creative,
    SyntheticControl,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Relation {
    Causes,
    Enables,
    Supports,
    Grounds,
    Clarifies,
    Qualifies,
    Inhibits,
    Prevents,
    Contradicts,
    Undercuts,
    Counterexample,
    Prerequisite,
    PartOf,
    TransformsTo,
    AnalogousTo,
    Predicts,
    Measures,
    Implements,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Goal {
    Prove,
    Generalize,
    Explain,
    Transfer,
    TraceCause,
    AlterPremise,
    Estimate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Lane {
    Deduction,
    Induction,
    Abduction,
    Analogy,
    Causal,
    Counterfactual,
    Probabilistic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    SupportedPath,
    Provisional,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StackRule {
    EvidenceFirst,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Invalid(&'static str),
    DuplicateLink(String),
    Creative(String),
}

#[derive(Clone, Debug)]
pub struct Link {
    pub id: String,
    pub world: String,
    pub cause: String,
    pub effect: String,
    pub domain: String,
    pub target_domain: String,
    pub conditions: Vec<String>,
    pub exceptions: Vec<String>,
    pub scope: String,
    pub target_scope: String,
    pub source_ids: Vec<String>,
    pub relation: Relation,
    pub sign: Sign,
    pub class: EvidenceClass,
    pub target_class: EvidenceClass,
    pub confidence: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct Admission {
    pub receipt_id: String,
    pub links: Vec<Link>,
}

#[derive(Clone, Debug)]
pub struct Analogy {
    pub source_cause: String,
    pub source_effect: String,
    pub target_cause: String,
    pub target_effect: String,
}

#[derive(Clone, Debug)]
pub struct Question {
    pub id: String,
    pub goal: Goal,
    pub cause: Option<String>,
    pub effect: String,
    pub allowed_domains: Vec<String>,
    pub conditions: Vec<String>,
    pub removed_link: Option<String>,
    pub analogy: Option<Analogy>,
}

#[derive(Clone, Debug)]
pub struct Input {
    pub admission: Admission,
    pub question: Question,
}

#[derive(Clone, Debug)]
pub struct PathTrace {
    pub link_ids: Vec<String>,
    pub world_ids: Vec<String>,
    pub source_ids: Vec<String>,
    pub sign: Sign,
    pub strength: Option<f64>,
    pub established: bool,
}

#[derive(Clone, Debug)]
pub struct LaneResult {
    pub lane: Lane,
    pub status: Status,
    pub direction: Option<Sign>,
    pub paths: Vec<PathTrace>,
    pub hypotheses: Vec<String>,
    pub probability: Option<f64>,
    pub reason: &'static str,
}

#[derive(Clone, Debug)]
pub struct Report {
    pub receipt_id: String,
    pub question_id: String,
    pub lanes: Vec<LaneResult>,
    pub factual_claim_authority: bool,
}

impl Admission {
    pub fn validate(&self) -> Result<(), Error> {
        if self.receipt_id.trim().is_empty() {
            return Err(Error::Invalid("missing admission receipt"));
        }
        if self.links.len() > 256 {
            return Err(Error::Invalid("admission exceeds 256 links"));
        }
        self.links
            .iter()
            .try_fold(BTreeSet::new(), |ids, link| {
                if [
                    link.id.as_str(),
                    link.world.as_str(),
                    link.cause.as_str(),
                    link.effect.as_str(),
                    link.domain.as_str(),
                    link.target_domain.as_str(),
                ]
                .iter()
                .any(|part| part.trim().is_empty())
                    || link.source_ids.iter().any(|id| id.trim().is_empty())
                {
                    return Err(Error::Invalid("causal link has an empty typed field"));
                }
                if link
                    .confidence
                    .is_some_and(|c| !c.is_finite() || !(0.0..=1.0).contains(&c))
                {
                    return Err(Error::Invalid("causal confidence is outside [0, 1]"));
                }
                if ids.contains(&link.id) {
                    return Err(Error::DuplicateLink(link.id.clone()));
                }
                Ok(ids
                    .union(&BTreeSet::from([link.id.clone()]))
                    .cloned()
                    .collect())
            })
            .map(|_| ())
    }
}

impl Question {
    pub fn validate(&self) -> Result<(), Error> {
        if self.id.trim().is_empty() || self.effect.trim().is_empty() {
            return Err(Error::Invalid("question ID and effect are required"));
        }
        if matches!(
            self.goal,
            Goal::Prove | Goal::Generalize | Goal::TraceCause | Goal::AlterPremise | Goal::Estimate
        ) && self.cause.as_deref().map_or(true, |c| c.trim().is_empty())
        {
            return Err(Error::Invalid("this goal requires a cause"));
        }
        if self.goal == Goal::AlterPremise && self.removed_link.is_none() {
            return Err(Error::Invalid("counterfactual requires a removed link"));
        }
        if self.goal == Goal::Transfer && self.analogy.is_none() {
            return Err(Error::Invalid("analogy requires an explicit mapping"));
        }
        if self.goal == Goal::Transfer
            && self
                .analogy
                .as_ref()
                .is_some_and(|mapping| mapping.target_effect != self.effect)
        {
            return Err(Error::Invalid(
                "analogy target differs from requested effect",
            ));
        }
        Ok(())
    }
}

struct PathEngine;
impl PathEngine {
    fn paths(&self, q: &Question, edges: &[Link]) -> Result<Vec<PathTrace>, Error> {
        let Some(cause) = q.cause.as_deref() else {
            return Ok(vec![]);
        };
        let (raw, _) = self.walk(edges, cause, &q.effect, &[cause.to_owned()], &[], 8, 8192)?;
        Ok(raw
            .into_iter()
            .take(64)
            .map(|path| PathTrace {
                link_ids: path.iter().map(|edge| edge.id.clone()).collect(),
                world_ids: path.iter().map(|edge| edge.world.clone()).collect(),
                source_ids: path
                    .iter()
                    .flat_map(|edge| edge.source_ids.iter().cloned())
                    .collect(),
                sign: if path.iter().any(|edge| edge.sign == Sign::Neutral) {
                    Sign::Neutral
                } else if path
                    .iter()
                    .filter(|edge| edge.sign == Sign::Opposes)
                    .count()
                    % 2
                    == 0
                {
                    Sign::Supports
                } else {
                    Sign::Opposes
                },
                strength: path.iter().fold(Some(1.0_f64), |strength, edge| {
                    strength.and_then(|prior| edge.confidence.map(|current| prior.min(current)))
                }),
                established: path.iter().all(|edge| {
                    matches!(
                        edge.class,
                        EvidenceClass::Established | EvidenceClass::Formal
                    ) && matches!(
                        edge.target_class,
                        EvidenceClass::Established | EvidenceClass::Formal
                    )
                }),
            })
            .collect())
    }

    fn walk(
        &self,
        edges: &[Link],
        at: &str,
        target: &str,
        seen: &[String],
        trail: &[Link],
        depth: usize,
        budget: usize,
    ) -> Result<(Vec<Vec<Link>>, usize), Error> {
        if depth == 0 {
            return Ok((vec![], budget));
        }
        edges
            .iter()
            .filter(|edge| edge.cause == at && !seen.contains(&edge.effect))
            .try_fold((Vec::new(), budget), |(paths, remaining), edge| {
                if paths.len() >= 64 {
                    return Ok((paths, remaining));
                }
                if remaining == 0 {
                    return Err(Error::Invalid("path exploration budget exhausted"));
                }
                let next: Vec<Link> = trail.iter().cloned().chain([edge.clone()]).collect();
                let (branch, left) = if edge.effect == target {
                    (vec![next], remaining - 1)
                } else {
                    let visited: Vec<String> =
                        seen.iter().cloned().chain([edge.effect.clone()]).collect();
                    self.walk(
                        edges,
                        &edge.effect,
                        target,
                        &visited,
                        &next,
                        depth - 1,
                        remaining - 1,
                    )?
                };
                Ok((paths.into_iter().chain(branch).take(64).collect(), left))
            })
    }
}

pub fn paths(q: &Question, edges: &[Link]) -> Result<Vec<PathTrace>, Error> {
    PathEngine.paths(q, edges)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(id: &str, world: &str, from: &str, to: &str) -> Link {
        Link {
            id: id.into(),
            world: world.into(),
            cause: from.into(),
            effect: to.into(),
            domain: "physical".into(),
            target_domain: "physical".into(),
            conditions: vec![],
            exceptions: vec![],
            scope: "test".into(),
            target_scope: "test".into(),
            source_ids: vec![id.into()],
            relation: Relation::Causes,
            sign: Sign::Supports,
            class: EvidenceClass::Provisional,
            target_class: EvidenceClass::Provisional,
            confidence: None,
        }
    }

    #[test]
    fn path_composes_worlds_without_fabricating_strength() {
        let links = vec![
            edge("a", "w1", "A", "B"),
            edge("b", "w2", "B", "C"),
            edge("cycle", "w3", "C", "A"),
        ];
        let q = Question {
            id: "test".into(),
            goal: Goal::TraceCause,
            cause: Some("A".into()),
            effect: "C".into(),
            allowed_domains: vec![],
            conditions: vec![],
            removed_link: None,
            analogy: None,
        };
        let paths = paths(&q, &links).expect("bounded path");
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].world_ids, ["w1", "w2"]);
        assert_eq!(paths[0].strength, None);
        assert!(!paths[0].established);
    }

    #[test]
    fn admission_rejects_duplicate_and_out_of_range_laws() {
        let first = edge("same", "w1", "A", "B");
        let duplicate = edge("same", "w2", "B", "C");
        assert_eq!(
            Admission {
                receipt_id: "test".into(),
                links: vec![first.clone(), duplicate]
            }
            .validate()
            .err(),
            Some(Error::DuplicateLink("same".into()))
        );
        let invalid = Link {
            confidence: Some(1.5),
            ..first
        };
        assert_eq!(
            Admission {
                receipt_id: "test".into(),
                links: vec![invalid]
            }
            .validate()
            .err(),
            Some(Error::Invalid("causal confidence is outside [0, 1]"))
        );
    }
}
