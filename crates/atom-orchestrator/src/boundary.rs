//! The output-side boundary: what may cross back, and why.
//!
//! Engine-side on purpose. The Lightcone is passive geometry and holds no
//! verdict; every verdict in this file belongs to the Atom side acting through
//! the boundary, exactly as the input side carries no verdict and the
//! post-weights sampler carries none.
//!
//! Quantities stay separate (anti-collapse): the claim's polarity, its
//! epistemic class, the engine's own lane status, and the release decision are
//! distinct fields. None of them is blended with another, and none is promoted
//! to factual confidence — `factual_claim_authority` is false on every path.
//!
//! The marker lists are bounded and explicit. They are a diagnostic
//! instrument, not a semantic judge: a text that uses none of them is
//! `NotApplicable`, and the check reports exactly which markers fired so the
//! coverage of the instrument is visible rather than assumed.

use crate::{Admission, EvidenceClass, Input, Report, Sign, Status};

/// What the text does with the admitted claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutVerdict {
    /// Uses the claim in the claim's own direction.
    Consistent,
    /// Asserts the opposite of the claim's polarity.
    Contradicts,
    /// Asserts a directed relation that this admission does not cover.
    Ungrounded,
    /// Neither asserts nor negates it.
    NotApplicable,
}

/// The boundary decision. Withholding is the E18 behaviour: the wrong verdict
/// does not cross. The raw text is still recorded by the caller — evidence is
/// never deleted, only marked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Release {
    Released,
    Withheld,
}

/// The text that crosses instead of a withheld generation.
pub const WITHHELD_TEXT: &str = "[withheld at the output boundary: the generation asserted the opposite of the admitted claim]";

#[derive(Clone, Debug)]
pub struct OutCheck {
    pub verdict: OutVerdict,
    pub release: Release,
    /// The admitted claim the check used, if this admission carried one.
    pub claim: Option<String>,
    pub claim_polarity: Option<Sign>,
    /// The claim's evidence class — reported, never folded into the verdict.
    pub claim_class: Option<EvidenceClass>,
    /// The engine's own lane position — reported, never folded in.
    pub engine_status: Option<Status>,
    pub engine_direction: Option<Sign>,
    /// Which markers fired, verbatim.
    pub asserted: Vec<&'static str>,
    pub negated: Vec<&'static str>,
    pub reason: &'static str,
    /// Always false: the boundary holds no factual authority.
    pub factual_claim_authority: bool,
}

/// Directed-relation markers: the text claims that one thing brings about,
/// explains, or requires another.
const ASSERT_MARKERS: &[&str] = &[
    "because",
    "causes",
    "caused",
    "leads to",
    "led to",
    "results in",
    "produced",
    "produces",
    "enables",
    "drives",
    "due to",
    "so that",
    "ensuring",
    "allows",
    "requires",
    "makes possible",
];

/// Markers that deny a relation outright.
const NEGATE_MARKERS: &[&str] = &[
    "no relationship",
    "no relation",
    "not related",
    "unrelated",
    "independent of",
    "does not cause",
    "doesn't cause",
    "does not lead",
    "does not produce",
    "no causal",
    "cannot cause",
    "never causes",
    "no evidence",
    "not causally",
    "no link",
    "not linked",
    "nothing to do with",
];

/// Test one marker as a whole phrase on word boundaries. Zero-dependency and
/// deterministic: no stemming, no scoring, no thresholds.
fn mentions(hay: &str, needle: &str) -> bool {
    let mut from = 0;
    while let Some(offset) = hay[from..].find(needle) {
        let start = from + offset;
        let end = start + needle.len();
        let before_ok = hay[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric());
        let after_ok = hay[end..].chars().next().is_none_or(|c| !c.is_alphanumeric());
        if before_ok && after_ok {
            return true;
        }
        from = start + hay[start..].chars().next().map_or(1, char::len_utf8);
        if from >= hay.len() {
            break;
        }
    }
    false
}

fn fired(text: &str, markers: &'static [&'static str]) -> Vec<&'static str> {
    markers
        .iter()
        .copied()
        .filter(|marker| mentions(text, marker))
        .collect()
}

/// The claim in question: the admitted link that carries the question's
/// cause into its effect. The admission is already the relevant neighbourhood,
/// so this is a lookup, not a judgement.
fn claim<'a>(admission: &'a Admission, input: &Input) -> Option<&'a crate::Link> {
    let cause = input.question.cause.as_deref()?;
    admission
        .links
        .iter()
        .find(|link| link.cause == cause && link.effect == input.question.effect)
}

/// The engine's own position, taken from its strongest lane. Reported beside
/// the check so a caller can see whether the engine and the text agreed;
/// never merged with the verdict.
fn engine_position(report: &Report) -> (Option<Status>, Option<Sign>) {
    let strongest = report
        .lanes
        .iter()
        .filter(|lane| lane.status != Status::Unknown)
        .max_by_key(|lane| match lane.status {
            Status::SupportedPath => 2,
            Status::Provisional => 1,
            Status::Unknown => 0,
        });
    match strongest {
        Some(lane) => (Some(lane.status), lane.direction),
        None => (report.lanes.first().map(|lane| lane.status), None),
    }
}

pub fn check_release(input: &Input, report: &Report, text: &str) -> OutCheck {
    let asserted = fired(text, ASSERT_MARKERS);
    let negated = fired(text, NEGATE_MARKERS);
    let link = claim(&input.admission, input);
    let (engine_status, engine_direction) = engine_position(report);
    let polarity = link.map(|link| link.sign);
    let mut check = OutCheck {
        verdict: OutVerdict::NotApplicable,
        release: Release::Released,
        claim: link.map(|link| link.id.clone()),
        claim_polarity: polarity,
        claim_class: link.map(|link| link.class),
        engine_status,
        engine_direction,
        asserted,
        negated,
        reason: "the text neither asserts nor denies the admitted claim",
        factual_claim_authority: false,
    };
    match polarity {
        Some(Sign::Supports) => {
            if !check.negated.is_empty() {
                check.verdict = OutVerdict::Contradicts;
                check.release = Release::Withheld;
                check.reason = "the text denies a claim the admission supports";
            } else if !check.asserted.is_empty() {
                check.verdict = OutVerdict::Consistent;
                check.reason = "the text uses the admitted claim in its own direction";
            }
        }
        Some(Sign::Opposes) => {
            if !check.asserted.is_empty() {
                check.verdict = OutVerdict::Contradicts;
                check.release = Release::Withheld;
                check.reason = "the text asserts a claim the admission opposes";
            } else if !check.negated.is_empty() {
                check.verdict = OutVerdict::Consistent;
                check.reason = "the text denies a claim the admission opposes";
            }
        }
        // No directional claim: an admitted-neutral link, or a question this
        // admission does not cover. Asserting a direction then has no ground.
        Some(Sign::Neutral) | None => {
            if !check.asserted.is_empty() {
                check.verdict = OutVerdict::Ungrounded;
                check.reason = "the text asserts a directed relation this admission does not cover";
            }
        }
    }
    check
}

/// Verified-claim feedback: only a consistent use of an admitted claim is
/// reinforced. Ungrounded assertions, withheld contradictions, and the
/// unknown/out-of-cone controls reinforce nothing — an admission that admitted
/// nothing cannot teach the engine anything.
pub fn reinforces(check: &OutCheck) -> bool {
    check.verdict == OutVerdict::Consistent
}

/// The text that crosses the boundary. The raw generation is still recorded by
/// the caller; this is only what a consumer downstream may use.
pub fn released_text<'a>(check: &OutCheck, raw: &'a str) -> &'a str {
    match check.release {
        Release::Released => raw,
        Release::Withheld => WITHHELD_TEXT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Analogy, Goal, Lane, LaneResult, Question, Relation};
    use atom_reasoning_engine::PathTrace;

    fn link(id: &str, sign: Sign, class: EvidenceClass) -> crate::Link {
        crate::Link {
            id: id.into(),
            world: "network-security-browser-v0".into(),
            cause: "net.tcp-ack".into(),
            effect: "net.retransmission".into(),
            domain: "networking".into(),
            target_domain: "networking".into(),
            conditions: vec![],
            exceptions: vec![],
            scope: "networking, cybersecurity, and browser-platform foundation tranche".into(),
            target_scope: "networking, cybersecurity, and browser-platform foundation tranche".into(),
            source_ids: vec!["rfc9293".into()],
            relation: Relation::Prerequisite,
            sign,
            class,
            target_class: class,
            confidence: None,
        }
    }

    fn input(links: Vec<crate::Link>) -> Input {
        Input {
            admission: Admission {
                receipt_id: "receipt".into(),
                links,
            },
            question: Question {
                id: "q".into(),
                goal: Goal::Prove,
                cause: Some("net.tcp-ack".into()),
                effect: "net.retransmission".into(),
                allowed_domains: vec![],
                conditions: vec![],
                removed_link: None,
                analogy: None::<Analogy>,
            },
        }
    }

    fn report(status: Status, direction: Option<Sign>) -> Report {
        Report {
            receipt_id: "receipt".into(),
            question_id: "q".into(),
            lanes: vec![LaneResult {
                lane: Lane::Deduction,
                status,
                direction,
                paths: vec![PathTrace {
                    link_ids: vec!["nsb.relation.net-ack-retransmit".into()],
                    world_ids: vec!["network-security-browser-v0".into()],
                    source_ids: vec!["rfc9293".into()],
                    sign: Sign::Supports,
                    strength: None,
                    established: false,
                }],
                hypotheses: vec![],
                probability: None,
                reason: "typed path only; unknown without established warrant",
            }],
            factual_claim_authority: false,
        }
    }

    /// The real generation from the first development case.
    const CASE_ONE: &str = "Candidate generation produces a diverse pool of \
        potential options, which then serves as the foundational input for \
        creative selection to identify and refine the most compelling ones.";
    /// The real generation from the unknown-term control.
    const CASE_TWO: &str = "It means nothing, as it is a fabricated term with no \
        established definition or function.";
    /// The real generation from the constraint-path control.
    const CASE_THREE: &str = "Hard constraints are applied first to eliminate \
        impossible options, ensuring that the subsequent creative process \
        focuses only on viable solutions that meet non-negotiable requirements.";

    #[test]
    fn a_claim_used_in_its_own_direction_is_consistent_and_reinforces() {
        let input = input(vec![link("nsb.relation.net-ack-retransmit", Sign::Supports, EvidenceClass::Creative)]);
        let check = check_release(&input, &report(Status::Unknown, None), CASE_ONE);
        assert_eq!(check.verdict, OutVerdict::Consistent);
        assert_eq!(check.release, Release::Released);
        assert!(check.asserted.contains(&"produces"));
        assert_eq!(check.claim_class, Some(EvidenceClass::Creative));
        // The engine's abstention is reported, not folded into the verdict.
        assert_eq!(check.engine_status, Some(Status::Unknown));
        assert!(reinforces(&check));
        assert!(!check.factual_claim_authority);
    }

    #[test]
    fn denying_a_supported_claim_is_withheld() {
        let input = input(vec![link("nsb.relation.net-ack-retransmit", Sign::Supports, EvidenceClass::Established)]);
        let check = check_release(
            &input,
            &report(Status::SupportedPath, Some(Sign::Supports)),
            "There is no relationship between candidate generation and creative selection.",
        );
        assert_eq!(check.verdict, OutVerdict::Contradicts);
        assert_eq!(check.release, Release::Withheld);
        assert_eq!(released_text(&check, "raw"), WITHHELD_TEXT);
        assert!(!reinforces(&check));
    }

    #[test]
    fn asserting_an_opposed_claim_is_withheld() {
        let input = input(vec![link("nsb.relation.net-ack-retransmit", Sign::Opposes, EvidenceClass::Established)]);
        let check = check_release(&input, &report(Status::SupportedPath, Some(Sign::Opposes)), CASE_ONE);
        assert_eq!(check.verdict, OutVerdict::Contradicts);
        assert_eq!(check.release, Release::Withheld);
    }

    #[test]
    fn denying_an_opposed_claim_is_consistent() {
        let input = input(vec![link("nsb.relation.net-ack-retransmit", Sign::Opposes, EvidenceClass::Established)]);
        let check = check_release(
            &input,
            &report(Status::SupportedPath, Some(Sign::Opposes)),
            "Candidate generation is not causally related to creative selection.",
        );
        assert_eq!(check.verdict, OutVerdict::Consistent);
        assert!(reinforces(&check));
    }

    #[test]
    fn the_unknown_control_asserts_nothing_and_reinforces_nothing() {
        let input = input(vec![]);
        let check = check_release(&input, &report(Status::Unknown, None), CASE_TWO);
        assert_eq!(check.verdict, OutVerdict::NotApplicable);
        assert_eq!(check.release, Release::Released);
        assert!(check.claim.is_none());
        assert!(!reinforces(&check));
    }

    #[test]
    fn the_out_of_cone_control_asserts_an_ungrounded_relation() {
        let input = input(vec![]);
        let check = check_release(&input, &report(Status::Unknown, None), CASE_THREE);
        assert_eq!(check.verdict, OutVerdict::Ungrounded);
        // Released — the model may know this independently — but never
        // reinforced, and the flag says it was not grounded by this admission.
        assert_eq!(check.release, Release::Released);
        assert!(!reinforces(&check));
    }

    #[test]
    fn a_neutral_claim_cannot_ground_a_direction() {
        let input = input(vec![link("nsb.relation.net-ack-retransmit", Sign::Neutral, EvidenceClass::Creative)]);
        let check = check_release(&input, &report(Status::Unknown, None), CASE_ONE);
        assert_eq!(check.verdict, OutVerdict::Ungrounded);
        assert!(!reinforces(&check));
    }

    #[test]
    fn markers_respect_word_boundaries() {
        assert!(!mentions("a makeshift solution", "makes"));
        assert!(mentions("space makes the menu hard", "makes"));
        assert!(mentions("This causes drift.", "causes"));
        assert!(!mentions("It showcases drift.", "causes"));
    }
}
