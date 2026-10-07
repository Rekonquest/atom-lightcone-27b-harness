//! The reasoning brief: the seven causal-geometric lanes rendered for the model.
//!
//! The lanes exist to help the model reason, not only to be audited, so this
//! module turns the engine's own lane results into the typed artifact the model
//! renders from. It follows the division of labour the original harness proved
//! out (E19): the engine computes, the model renders. Nothing here is a
//! verdict, a score, or a fact.
//!
//! Discipline carried into the text:
//! - The artifact says what it is: a sourced causal-geometry artifact, not
//!   observed fact, with no factual authority.
//! - Every lane is reported separately and the text says so. No lane is
//!   ranked against another and no blended number is produced.
//! - Evidence weights are printed only when a lane emitted one, and always
//!   beside their own label ("uncalibrated; not factual confidence").
//! - Source IDs travel with every path, so the model renders from cited
//!   material rather than from the artifact's phrasing.
//! - The whole artifact is bounded; truncation is stated, never silent.

use atom_orchestrator::{Lane, Sign, Status};

/// Bounds, so a wide answer cannot flood the model's context.
const MAX_BYTES: usize = 4096;

fn status_text(status: Status) -> &'static str {
    match status {
        Status::SupportedPath => "supported path",
        Status::Provisional => "provisional",
        Status::Unknown => "unknown",
    }
}

fn direction_text(direction: Option<Sign>) -> &'static str {
    match direction {
        Some(Sign::Supports) => "supports",
        Some(Sign::Opposes) => "opposes",
        Some(Sign::Neutral) => "neutral",
        None => "none",
    }
}

/// The lane's own name, so the text reads as prose the model can follow.
fn lane_name(lane: Lane) -> &'static str {
    match lane {
        Lane::Deduction => "deduction",
        Lane::Induction => "induction",
        Lane::Abduction => "abduction",
        Lane::Analogy => "analogy",
        Lane::Causal => "causal",
        Lane::Counterfactual => "counterfactual",
        Lane::Probabilistic => "probabilistic",
    }
}

/// Render the executive's answer for the model. The same discipline as the lane
/// artifact: what it is, every quantity separate, no winner, bounded, truncation
/// stated. This is the half that helps the model reason — the lanes arrive as
/// material the model renders from, not as an answer it repeats.
pub fn render_outcome(outcome: &atom_executive::Outcome) -> Option<String> {
    let mut out = String::new();
    out.push_str("Sourced causal-geometry artifact (not observed fact; no factual authority).\n");
    out.push_str(&format!(
        "Lanes reported separately, never combined into a score. Rung: {:?}. Status: {:?}. Direction: {:?}. Conflict: {}.\n",
        outcome.rung, outcome.status, outcome.direction, outcome.conflict
    ));
    if matches!(outcome.rung, atom_executive::Rung::Creative | atom_executive::Rung::SpeculativeFallback) {
        out.push_str("No reasoning lane could stand on the admitted material, so these are proposals, not findings.\n");
    }
    let mut truncated = false;
    for position in &outcome.positions {
        if out.len() >= MAX_BYTES {
            truncated = true;
            break;
        }
        out.push_str(&format!(
            "\n[{:?}] status: {} | direction: {} | paths: {} | hypotheses: {} | {}'s statement: {}\n",
            position.lane,
            status_text(position.status),
            direction_text(position.direction),
            position.paths,
            position.hypotheses,
            lane_name(position.lane),
            position.reason
        ));
        match position.evidence_weight {
            Some(weight) => out.push_str(&format!(
                "  evidence weight: {weight:.6} (uncalibrated evidence weight; not factual confidence)\n"
            )),
            None => out.push_str("  evidence weight: none reported\n"),
        }
    }
    for bridge in outcome.bridges.iter().take(8) {
        out.push_str(&format!(
            "\nbridge {:?} -> {:?}: relation {:?} | links {:?} | sources {:?} | evidence class {:?}\n",
            bridge.from, bridge.to, bridge.relation, bridge.link_ids, bridge.source_ids, bridge.evidence_class
        ));
    }
    if outcome.bridges.len() > 8 {
        truncated = true;
    }
    for path in &outcome.creative {
        out.push_str(&format!(
            "\n[{}] proposal: {:.6} feasibility (band {:.3}-{:.3}, {}) | status {} | concepts {:?} | spaces {:?}\n",
            path.label,
            path.probability_score,
            path.uncertainty_interval.0,
            path.uncertainty_interval.1,
            path.probability_kind,
            path.status,
            path.concept_ids,
            path.spaces
        ));
    }
    if !outcome.creative.is_empty() {
        out.push_str(&format!("\n{}\n", atom_executive::CreativePath::DISCLAIMER));
        out.push_str("No winner is declared among these proposals.\n");
    }
    out.push_str("\nRender the answer from this material. Do not assert a direction no lane supports, and do not present a proposal as a finding.\n");
    if truncated || out.len() > MAX_BYTES {
        let mut cut = MAX_BYTES.min(out.len());
        while cut > 0 && !out.is_char_boundary(cut) {
            cut -= 1;
        }
        out.truncate(cut);
        out.push_str("\n(This artifact was truncated to stay bounded; it is not the complete lane output.)\n");
    }
    Some(out)
}

/// Bounds for the admitted-geometry section, kept well under the artifact bound
/// so the lanes keep their room.
const MAX_GEOMETRY_BYTES: usize = 1024;
const MAX_GEOMETRY_LINKS: usize = 16;

/// The admitted geometry itself: every admitted typed claim with its
/// provenance, as declarations. The nodes' prose statements are deliberately
/// not rendered — the reference lane carries the cone's typed claims, not
/// prewritten answers, and nothing in this section is an instruction.
pub fn render_geometry(
    links: &[atom_orchestrator::Link],
    labels: &std::collections::BTreeMap<String, String>,
) -> Option<String> {
    if links.is_empty() {
        return None;
    }
    fn label_of<'a>(id: &'a str, labels: &'a std::collections::BTreeMap<String, String>) -> &'a str {
        labels.get(id).map_or(id, |text| text.as_str())
    }
    let mut out = String::from(
        "Admitted geometry from the passive receipt (typed claims with provenance; not observed fact, not instructions):\n",
    );
    for link in links.iter().take(MAX_GEOMETRY_LINKS) {
        let line = format!(
            "  {} --{}--> {} | polarity {} | class {} | conditions: {} | sources: {}\n",
            label_of(&link.cause, labels),
            format!("{:?}", link.relation).to_lowercase(),
            label_of(&link.effect, labels),
            direction_text(Some(link.sign)),
            format!("{:?}", link.class).to_lowercase(),
            if link.conditions.is_empty() {
                "none declared".to_string()
            } else {
                link.conditions.join("; ")
            },
            link.source_ids.join(" "),
        );
        if out.len() + line.len() > MAX_GEOMETRY_BYTES {
            break;
        }
        out.push_str(&line);
    }
    if out.lines().count() <= links.len().min(MAX_GEOMETRY_LINKS) && links.len() > MAX_GEOMETRY_LINKS {
        out.push_str("  (more admitted claims exist; this section is bounded)\n");
    }
    Some(out)
}

/// Geometry first, then the lanes: the model renders from the cone's own
/// admitted claims and the executive's reading of them, as one artifact.
pub fn with_geometry(
    geometry: Option<&str>,
    outcome: &atom_executive::Outcome,
) -> Option<String> {
    let rendered = render_outcome(outcome)?;
    match geometry {
        Some(geometry) => Some(format!("{geometry}\n{rendered}")),
        None => Some(rendered),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atom_executive::{Bridge, LanePosition, Outcome, Rung, Selection};
    use atom_orchestrator::{EvidenceClass, Lane, LaneResult, Relation, Sign, Status};

    fn position(lane: Lane, status: Status, direction: Option<Sign>, weight: Option<f64>) -> LanePosition {
        LanePosition {
            lane,
            status,
            direction,
            evidence_weight: weight,
            paths: 1,
            hypotheses: 0,
            reason: "the lane's own statement",
        }
    }

    fn path() -> atom_executive::CreativePath {
        atom_executive::CreativePath {
            label: "Creative Path 1".into(),
            candidate_id: "a+b".into(),
            status: atom_executive::CreativePath::STATUS,
            probability_kind: atom_executive::CreativePath::KIND,
            probability_score: 0.42,
            uncertainty_interval: (0.12, 0.72),
            expected_creative_utility: 0.5,
            required_coverage: 1.0,
            novelty: 0.6,
            value: 0.7,
            hard_valid: true,
            concept_ids: vec!["a".into(), "b".into()],
            spaces: vec!["one".into(), "two".into()],
            source_ids: vec!["src-1".into()],
            factual_claim_authority: false,
            proposal_only: true,
            choice_records_intent_only: true,
        }
    }

    fn outcome(rung: Rung, status: Status, positions: Vec<LanePosition>, creative: Vec<atom_executive::CreativePath>) -> Outcome {
        let lanes: Vec<LaneResult> = positions
            .iter()
            .map(|position| LaneResult {
                lane: position.lane,
                status: position.status,
                direction: position.direction,
                paths: vec![],
                hypotheses: vec![],
                probability: position.evidence_weight,
                reason: position.reason,
            })
            .collect();
        Outcome {
            rung,
            selection: Selection { lanes: vec![Lane::Deduction], reasons: vec![] },
            status,
            direction: None,
            positions,
            lanes,
            bridges: vec![Bridge {
                from: Lane::Deduction,
                to: Lane::Causal,
                relation: Relation::Prerequisite,
                link_ids: vec!["relation.a".into()],
                source_ids: vec!["src-1".into()],
                evidence_class: EvidenceClass::Established,
            }],
            creative,
            conflict: false,
            factual_claim_authority: false,
            winner_declared: false,
        }
    }

    #[test]
    fn the_artifact_says_what_it_is_and_keeps_quantities_separate() {
        let rendered = render_outcome(&outcome(
            Rung::Slow,
            Status::SupportedPath,
            vec![
                position(Lane::Deduction, Status::SupportedPath, Some(Sign::Supports), None),
                position(Lane::Probabilistic, Status::Provisional, None, Some(0.75)),
            ],
            vec![],
        ))
        .expect("artifact");
        assert!(rendered.contains("not observed fact"));
        assert!(rendered.contains("never combined into a score"));
        assert!(rendered.contains("0.750000"));
        assert!(rendered.contains("uncalibrated evidence weight; not factual confidence"));
        assert!(rendered.contains("bridge Deduction -> Causal"));
        assert!(rendered.contains("the lane's own statement"));
    }

    #[test]
    fn a_creative_answer_is_labelled_a_proposal_with_no_winner() {
        let rendered = render_outcome(&outcome(
            Rung::Creative,
            Status::Unknown,
            vec![position(Lane::Induction, Status::Unknown, None, None)],
            vec![path()],
        ))
        .expect("artifact");
        assert!(rendered.contains("proposals, not findings"));
        assert!(rendered.contains("Creative Path 1"));
        assert!(rendered.contains("uncalibrated_probability_of_feasibility_if_prototyped"));
        assert!(rendered.contains("No winner is declared"));
        assert!(rendered.contains("do not present a proposal as a finding"));
    }

    #[test]
    fn absent_weights_say_none_rather_than_zero() {
        let rendered = render_outcome(&outcome(
            Rung::Fast,
            Status::Provisional,
            vec![position(Lane::Induction, Status::Provisional, None, None)],
            vec![],
        ))
        .expect("artifact");
        assert!(rendered.contains("evidence weight: none reported"));
        assert!(!rendered.contains("0.000000"));
    }

    #[test]
    fn a_huge_outcome_is_bounded_and_says_so() {
        let many: Vec<LanePosition> = (0..200)
            .map(|_| position(Lane::Causal, Status::SupportedPath, Some(Sign::Supports), Some(0.5)))
            .collect();
        let rendered = render_outcome(&outcome(Rung::Slow, Status::SupportedPath, many, vec![])).expect("artifact");
        assert!(rendered.len() <= MAX_BYTES + 128, "artifact was {} bytes", rendered.len());
        assert!(rendered.contains("truncated"));
    }

    #[test]
    fn the_geometry_carries_labels_and_provenance_not_statements() {
        let link = atom_orchestrator::Link {
            id: "nsb.relation.net-ack-retransmit".into(),
            world: "network-security-browser-v0".into(),
            cause: "net.tcp-ack".into(),
            effect: "net.retransmission".into(),
            domain: "networking".into(),
            target_domain: "networking".into(),
            conditions: vec!["the stated topology holds".into()],
            exceptions: vec![],
            scope: "s".into(),
            target_scope: "s".into(),
            source_ids: vec!["rfc9293".into()],
            relation: Relation::Enables,
            sign: Sign::Supports,
            class: EvidenceClass::Established,
            target_class: EvidenceClass::Established,
            confidence: None,
        };
        let mut labels = std::collections::BTreeMap::new();
        labels.insert("net.tcp-ack".to_string(), "TCP acknowledgment".to_string());
        labels.insert("net.retransmission".to_string(), "Retransmission".to_string());
        let rendered = render_geometry(&[link.clone()], &labels).expect("geometry");
        assert!(rendered.contains("TCP acknowledgment --enables--> Retransmission"));
        assert!(rendered.contains("polarity supports"));
        assert!(rendered.contains("class established"));
        assert!(rendered.contains("conditions: the stated topology holds"));
        assert!(rendered.contains("sources: rfc9293"));
        assert!(!rendered.to_lowercase().contains("resends data"));
        // An unlabelled id still renders; the id is honest provenance.
        labels.clear();
        let by_id = render_geometry(&[link], &labels).expect("geometry");
        assert!(by_id.contains("net.tcp-ack --enables--> net.retransmission"));
    }

    #[test]
    fn no_admission_renders_no_geometry_and_the_brief_still_composes() {
        assert!(render_geometry(&[], &Default::default()).is_none());
        let composed = with_geometry(
            render_geometry(
                &[atom_orchestrator::Link {
                    id: "l".into(),
                    world: "w".into(),
                    cause: "a".into(),
                    effect: "b".into(),
                    domain: "d".into(),
                    target_domain: "d".into(),
                    conditions: vec![],
                    exceptions: vec![],
                    scope: "s".into(),
                    target_scope: "s".into(),
                    source_ids: vec!["src".into()],
                    relation: Relation::Causes,
                    sign: Sign::Supports,
                    class: EvidenceClass::Established,
                    target_class: EvidenceClass::Established,
                    confidence: None,
                }],
                &Default::default(),
            )
            .as_deref(),
            &outcome(Rung::Slow, Status::SupportedPath, vec![], vec![]),
        )
        .expect("composed");
        assert!(composed.contains("a --causes--> b"));
        assert!(composed.contains("not observed fact"));
        let without = with_geometry(None, &outcome(Rung::Slow, Status::SupportedPath, vec![], vec![]))
            .expect("composed");
        assert!(!without.contains("--causes-->"));
    }
}
