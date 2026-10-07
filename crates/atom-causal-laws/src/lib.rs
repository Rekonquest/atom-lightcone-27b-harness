//! Sourced CausalLaws are distinct from passive Lightcone edges.

pub use atom_causal_primitives::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub struct CausalLaw {
    pub id: String,
    pub domain: String,
    pub cause: String,
    pub effect: String,
    pub direction: i8,
    pub magnitude_mean: f64,
    pub magnitude_m2: f64,
    pub effect_variance_mean: f64,
    pub delay_mean: f64,
    pub invariant_error_mean: f64,
    pub support: u64,
    pub contradictions: u64,
    pub treated_worlds: u64,
    pub baseline_worlds: u64,
    pub confidence: f64,
    pub persistence: f64,
    pub phase: f64,
    pub status: String,
    pub atom_program: Vec<String>,
    pub source_law_ids: Vec<String>,
    pub contexts: BTreeMap<String, u64>,
    pub provenance_hashes: Vec<String>,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct LawWorld {
    pub id: String,
    pub model_hash: String,
    pub reliability: f64,
    pub laws: Vec<CausalLaw>,
}

#[derive(Clone, Debug)]
pub struct LawProblem {
    pub cause: String,
    pub effect: String,
    pub domain: Option<String>,
    pub conditions: Vec<String>,
    pub allowed_domains: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LawPath {
    pub law_ids: Vec<String>,
    pub world_ids: Vec<String>,
    pub domain_path: Vec<String>,
    pub condition_path: Vec<Vec<String>>,
    pub feature_path: Vec<String>,
    pub provenance_hashes: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub direction: i8,
    pub confidence: f64,
    pub persistence: f64,
    pub strength: f64,
    pub cross_world: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorldRoute {
    pub world_id: String,
    pub route_score: f64,
    pub exact_laws: usize,
    pub broad_laws: usize,
    pub composed_paths: usize,
    pub domain_fit: f64,
    pub evidence_quality: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LawPlan {
    pub strategy: &'static str,
    pub selected: Vec<WorldRoute>,
    pub rejected: Vec<WorldRoute>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LawError {
    Invalid(&'static str),
    Duplicate(String),
    BudgetExhausted,
}

#[derive(Clone, Debug)]
pub struct LawRegistry {
    worlds: Vec<LawWorld>,
}

impl CausalLaw {
    pub fn validate(&self) -> Result<(), LawError> {
        if [
            self.id.as_str(),
            self.domain.as_str(),
            self.cause.as_str(),
            self.effect.as_str(),
            self.status.as_str(),
        ]
        .iter()
        .any(|part| part.trim().is_empty())
        {
            return Err(LawError::Invalid("causal law has empty identity"));
        }
        if !matches!(self.direction, -1 | 1)
            || !matches!(self.status.as_str(), "crystallized" | "forming" | "retired")
        {
            return Err(LawError::Invalid(
                "causal law has invalid direction or status",
            ));
        }
        if ![self.confidence, self.persistence]
            .iter()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
        {
            return Err(LawError::Invalid(
                "causal law confidence or persistence invalid",
            ));
        }
        if ![
            self.magnitude_mean,
            self.magnitude_m2,
            self.effect_variance_mean,
            self.delay_mean,
            self.invariant_error_mean,
            self.phase,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            return Err(LawError::Invalid("causal law numeric field invalid"));
        }
        if self
            .provenance_hashes
            .iter()
            .chain(&self.evidence_ids)
            .any(|part| part.trim().is_empty())
        {
            return Err(LawError::Invalid("causal law source identity missing"));
        }
        if self.status == "crystallized"
            && (self.provenance_hashes.is_empty() || self.evidence_ids.is_empty())
        {
            return Err(LawError::Invalid("crystallized law lacks source evidence"));
        }
        Ok(())
    }
}

impl LawProblem {
    pub fn validate(&self) -> Result<(), LawError> {
        if self.cause.trim().is_empty() || self.effect.trim().is_empty() {
            return Err(LawError::Invalid("causal problem needs cause and effect"));
        }
        if self
            .conditions
            .iter()
            .chain(&self.allowed_domains)
            .any(|s| s.trim().is_empty())
        {
            return Err(LawError::Invalid("causal problem contains empty scope"));
        }
        Ok(())
    }
}

impl LawRegistry {
    pub fn new(mut worlds: Vec<LawWorld>) -> Result<Self, LawError> {
        let mut ids = BTreeSet::new();
        let mut models = BTreeSet::new();
        for world in &worlds {
            if world.id.trim().is_empty()
                || world.model_hash.len() != 64
                || !world.reliability.is_finite()
                || !(0.0..=1.0).contains(&world.reliability)
            {
                return Err(LawError::Invalid("world identity or reliability invalid"));
            }
            if !ids.insert(world.id.clone()) || !models.insert(world.model_hash.clone()) {
                return Err(LawError::Duplicate(world.id.clone()));
            }
            let mut law_ids = BTreeSet::new();
            for law in &world.laws {
                law.validate()?;
                if !law_ids.insert(law.id.clone()) {
                    return Err(LawError::Duplicate(law.id.clone()));
                }
            }
        }
        worlds.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(Self { worlds })
    }

    pub fn worlds(&self) -> &[LawWorld] {
        &self.worlds
    }

    pub fn plan(&self, problem: &LawProblem) -> Result<LawPlan, LawError> {
        problem.validate()?;
        let paths = self.paths(problem)?;
        let mut routes: Vec<WorldRoute> = self
            .worlds
            .iter()
            .map(|world| {
                let broad: Vec<&CausalLaw> = world
                    .laws
                    .iter()
                    .filter(|law| {
                        law.cause == problem.cause
                            && law.effect == problem.effect
                            && problem
                                .domain
                                .as_ref()
                                .is_none_or(|domain| &law.domain == domain)
                            && law.status != "retired"
                    })
                    .collect();
                let exact = broad
                    .iter()
                    .filter(|law| {
                        problem
                            .conditions
                            .iter()
                            .all(|condition| law.contexts.contains_key(condition))
                    })
                    .count();
                let composed: Vec<&LawPath> = paths
                    .iter()
                    .filter(|path| {
                        path.law_ids.len() > 1 && path.world_ids.iter().all(|id| id == &world.id)
                    })
                    .collect();
                let domain_fit = if problem
                    .domain
                    .as_ref()
                    .is_none_or(|domain| world.laws.iter().any(|law| &law.domain == domain))
                {
                    1.0
                } else {
                    0.0
                };
                let evidence_quality = if broad.is_empty() && composed.is_empty() {
                    0.0
                } else {
                    (broad
                        .iter()
                        .map(|law| {
                            law.confidence
                                * law.persistence
                                * (1.0 - (-(law.support as f64) / 4.0).exp())
                        })
                        .sum::<f64>()
                        + composed.iter().map(|path| path.strength).sum::<f64>())
                        / (broad.len() + composed.len()) as f64
                };
                let score = if broad.is_empty() && composed.is_empty() {
                    0.0
                } else {
                    (0.36 * f64::from(exact > 0)
                        + 0.22 * f64::from(!broad.is_empty())
                        + 0.20 * f64::from(!composed.is_empty())
                        + 0.10 * domain_fit
                        + 0.12 * evidence_quality)
                        .min(1.0)
                };
                WorldRoute {
                    world_id: world.id.clone(),
                    route_score: score,
                    exact_laws: exact,
                    broad_laws: broad.len(),
                    composed_paths: composed.len(),
                    domain_fit,
                    evidence_quality,
                }
            })
            .collect();
        routes.sort_by(|a, b| {
            b.route_score
                .total_cmp(&a.route_score)
                .then_with(|| a.world_id.cmp(&b.world_id))
        });
        let selected: Vec<WorldRoute> = routes
            .iter()
            .filter(|route| route.route_score >= 0.08)
            .take(4)
            .cloned()
            .collect();
        let chosen: BTreeSet<&str> = selected
            .iter()
            .map(|route| route.world_id.as_str())
            .collect();
        let rejected = routes
            .into_iter()
            .filter(|route| !chosen.contains(route.world_id.as_str()))
            .collect();
        let strategy = match selected.as_slice() {
            [] => "active_world_search",
            [_, _, ..] => "multi_world_deliberation",
            [route] if route.composed_paths > 0 && route.exact_laws == 0 => {
                "decompositional_causal_chain"
            }
            [_] => "fast_causal_recall",
        };
        Ok(LawPlan {
            strategy,
            selected,
            rejected,
        })
    }

    pub fn paths(&self, problem: &LawProblem) -> Result<Vec<LawPath>, LawError> {
        problem.validate()?;
        let allowed: BTreeSet<&str> = problem
            .allowed_domains
            .iter()
            .map(String::as_str)
            .chain(
                problem
                    .domain
                    .as_deref()
                    .filter(|_| problem.allowed_domains.is_empty()),
            )
            .collect();
        let edges: Vec<(&LawWorld, &CausalLaw)> = self
            .worlds
            .iter()
            .flat_map(|world| world.laws.iter().map(move |law| (world, law)))
            .filter(|(_, law)| law.status == "crystallized")
            .filter(|(_, law)| allowed.is_empty() || allowed.contains(law.domain.as_str()))
            .filter(|(_, law)| {
                problem.conditions.is_empty()
                    || problem
                        .conditions
                        .iter()
                        .all(|condition| law.contexts.contains_key(condition))
            })
            .collect();
        let mut remaining = 8192;
        let mut found = Vec::new();
        let mut visited = BTreeSet::from([problem.cause.as_str()]);
        walk(
            &edges,
            &problem.cause,
            &problem.effect,
            &mut visited,
            &mut Vec::new(),
            &mut found,
            8,
            &mut remaining,
        )?;
        Ok(found)
    }
}

fn walk<'a>(
    edges: &[(&'a LawWorld, &'a CausalLaw)],
    at: &str,
    target: &str,
    visited: &mut BTreeSet<&'a str>,
    trail: &mut Vec<(&'a LawWorld, &'a CausalLaw)>,
    found: &mut Vec<LawPath>,
    depth: usize,
    remaining: &mut usize,
) -> Result<(), LawError> {
    if depth == 0 || found.len() >= 64 {
        return Ok(());
    }
    for (world, law) in edges.iter().filter(|(_, law)| law.cause == at) {
        if visited.contains(law.effect.as_str()) {
            continue;
        }
        if *remaining == 0 {
            return Err(LawError::BudgetExhausted);
        }
        *remaining -= 1;
        visited.insert(&law.effect);
        trail.push((world, law));
        if law.effect == target {
            found.push(path(trail));
        } else {
            walk(
                edges,
                &law.effect,
                target,
                visited,
                trail,
                found,
                depth - 1,
                remaining,
            )?;
        }
        trail.pop();
        visited.remove(law.effect.as_str());
        if found.len() >= 64 {
            break;
        }
    }
    Ok(())
}

fn path(trail: &[(&LawWorld, &CausalLaw)]) -> LawPath {
    let world_ids: Vec<String> = trail.iter().map(|(world, _)| world.id.clone()).collect();
    let mut provenance: BTreeSet<String> = BTreeSet::new();
    let mut evidence: BTreeSet<String> = BTreeSet::new();
    for (_, law) in trail {
        provenance.extend(law.provenance_hashes.iter().cloned());
        evidence.extend(law.evidence_ids.iter().cloned());
    }
    let confidence = trail
        .iter()
        .map(|(_, law)| law.confidence)
        .fold(1.0, f64::min);
    let persistence = trail
        .iter()
        .map(|(_, law)| law.persistence)
        .fold(1.0, f64::min);
    LawPath {
        law_ids: trail.iter().map(|(_, law)| law.id.clone()).collect(),
        world_ids: world_ids.clone(),
        domain_path: trail.iter().map(|(_, law)| law.domain.clone()).collect(),
        condition_path: trail
            .iter()
            .map(|(_, law)| law.contexts.keys().cloned().collect())
            .collect(),
        feature_path: trail
            .first()
            .map(|(_, law)| law.cause.clone())
            .into_iter()
            .chain(trail.iter().map(|(_, law)| law.effect.clone()))
            .collect(),
        provenance_hashes: provenance.into_iter().collect(),
        evidence_ids: evidence.into_iter().collect(),
        direction: trail.iter().map(|(_, law)| law.direction).product(),
        confidence,
        persistence,
        strength: confidence * persistence,
        cross_world: world_ids.iter().collect::<BTreeSet<_>>().len() > 1,
    }
}
