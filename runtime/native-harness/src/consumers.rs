//! Consumers for what the engine already computes: per-lane probability, the
//! prediction relation, and contested-evidence status.
//!
//! Anti-collapse law: every quantity below is reported beside the others and
//! none is blended. A lane's uncalibrated evidence weight is not a confidence,
//! a prediction path is not a causal path, and contested evidence is not a
//! score. `factual_claim_authority` stays false throughout.

use atom_orchestrator::{Admission, Lane, LaneResult, Relation, Sign, Status};

/// One lane's position, consumed and reported separately.
#[derive(Clone, Debug)]
pub struct LaneSummary {
    pub lane: Lane,
    pub status: Status,
    pub direction: Option<Sign>,
    /// The lane's own uncalibrated evidence weight, where the lane emits one.
    pub evidence_weight: Option<f64>,
    pub paths: usize,
    pub hypotheses: usize,
}

/// Consume the reasoning report's lanes: status, direction, evidence weight,
/// path count. Nothing is merged and nothing is recomputed.
pub fn lane_summaries(lanes: &[LaneResult]) -> Vec<LaneSummary> {
    lanes
        .iter()
        .map(|lane| LaneSummary {
            lane: lane.lane,
            status: lane.status,
            direction: lane.direction,
            evidence_weight: lane.probability,
            paths: lane.paths.len(),
            hypotheses: lane.hypotheses.len(),
        })
        .collect()
}

/// Does any lane carry an uncalibrated evidence weight? That is the engine's
/// own signal that the admitted evidence is contested, and it is consumed as a
/// flag, never as a number folded into anything else.
pub fn contested_evidence(lanes: &[LaneResult]) -> bool {
    lanes.iter().any(|lane| lane.probability.is_some())
}

/// A path the admission roots in the prediction relation, kept apart from the
/// causal paths.
#[derive(Clone, Debug)]
pub struct PredictionPath {
    pub link_ids: Vec<String>,
    pub source_ids: Vec<String>,
    pub sign: Sign,
    pub established: bool,
}

/// Consume the prediction relation: every reported path whose links are
/// `Predicts` edges. A prediction is not a cause, so it is reported as its own
/// list rather than mixed into the causal ones.
pub fn prediction_paths(admission: &Admission, lanes: &[LaneResult]) -> Vec<PredictionPath> {
    let predicts = |link_id: &str| {
        admission
            .links
            .iter()
            .find(|link| link.id == link_id)
            .is_some_and(|link| link.relation == Relation::Predicts)
    };
    lanes
        .iter()
        .flat_map(|lane| lane.paths.iter())
        .filter(|path| !path.link_ids.is_empty() && path.link_ids.iter().all(|id| predicts(id)))
        .map(|path| PredictionPath {
            link_ids: path.link_ids.clone(),
            source_ids: path.source_ids.clone(),
            sign: path.sign,
            established: path.established,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use atom_orchestrator::{Admission, EvidenceClass, LaneResult, Link, PathTrace};

    fn link(id: &str, relation: Relation, sign: Sign) -> Link {
        Link {
            id: id.into(),
            world: "w".into(),
            cause: "a".into(),
            effect: "b".into(),
            domain: "d".into(),
            target_domain: "d".into(),
            conditions: vec![],
            exceptions: vec![],
            scope: "s".into(),
            target_scope: "s".into(),
            source_ids: vec![format!("{id}-source")],
            relation,
            sign,
            class: EvidenceClass::Established,
            target_class: EvidenceClass::Established,
            confidence: None,
        }
    }

    fn trace(link_ids: &[&str], sign: Sign) -> PathTrace {
        PathTrace {
            link_ids: link_ids.iter().map(|id| (*id).into()).collect(),
            world_ids: vec!["w".into()],
            source_ids: vec!["a-source".into()],
            sign,
            strength: None,
            established: true,
        }
    }

    fn lanes(lanes: Vec<LaneResult>) -> Vec<LaneResult> {
        lanes
    }

    fn lane(name: Lane, status: Status, probability: Option<f64>, paths: Vec<PathTrace>) -> LaneResult {
        LaneResult {
            lane: name,
            status,
            direction: None,
            paths,
            hypotheses: vec![],
            probability,
            reason: "test",
        }
    }

    #[test]
    fn a_lane_weight_is_reported_and_never_merged() {
        let report = lanes(vec![lane(Lane::Probabilistic, Status::Provisional, Some(0.7), vec![])]);
        let summaries = lane_summaries(&report);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].evidence_weight, Some(0.7));
        assert_eq!(summaries[0].status, Status::Provisional);
        assert!(contested_evidence(&report));
    }

    #[test]
    fn absent_weight_is_not_zero_and_not_contested() {
        let report = lanes(vec![lane(Lane::Deduction, Status::Unknown, None, vec![])]);
        assert_eq!(lane_summaries(&report)[0].evidence_weight, None);
        assert!(!contested_evidence(&report));
    }

    #[test]
    fn prediction_paths_are_kept_apart_from_causal_paths() {
        let admission = Admission {
            receipt_id: "r".into(),
            links: vec![
                link("predicts", Relation::Predicts, Sign::Supports),
                link("causes", Relation::Causes, Sign::Supports),
            ],
        };
        let report = lanes(vec![lane(
            Lane::Induction,
            Status::Provisional,
            None,
            vec![
                trace(&["predicts"], Sign::Supports),
                trace(&["causes"], Sign::Supports),
                trace(&["predicts", "causes"], Sign::Supports),
            ],
        )]);
        let predictions = prediction_paths(&admission, &report);
        assert_eq!(predictions.len(), 1, "only the all-Predicts path qualifies");
        assert_eq!(predictions[0].link_ids, vec!["predicts"]);
        assert_eq!(predictions[0].source_ids, vec!["a-source"]);
    }

    #[test]
    fn a_question_without_a_lane_still_reports_nothing_rather_than_guessing() {
        let admission = Admission {
            receipt_id: "r".into(),
            links: vec![link("predicts", Relation::Predicts, Sign::Supports)],
        };
        let report = lanes(vec![]);
        assert!(lane_summaries(&report).is_empty());
        assert!(prediction_paths(&admission, &report).is_empty());
        assert!(!contested_evidence(&report));
    }
}
