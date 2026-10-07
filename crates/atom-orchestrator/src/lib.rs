//! Fixed Atom entrypoint; task-sensitive decisions belong to the reasoning engine.

pub mod boundary;

pub use atom_reasoning_engine::{
    Admission, Analogy, Brief, Candidate, CognitiveCreativeReport, Concept, CreativeConfig,
    CreativeError, CreativeInput, CreativeResult, Engine, Error, EvidenceClass, Frame, Goal, Input,
    Lane, LaneResult, LawError, LawProblem, LawRegistry, Link, Operation, PathTrace, Question,
    Relation, Report, Sign, SourcedLawReport, StackRule, Status, Tension,
};
pub use boundary::{OutCheck, OutVerdict, Release, check_release, reinforces, released_text};

const STACK_RULE: StackRule = StackRule::EvidenceFirst;

pub fn route() -> StackRule {
    STACK_RULE
}

pub fn run(engine: &Engine, input: Input) -> Result<Report, Error> {
    engine.dispatch(route(), input)
}

pub fn run_creative(
    engine: &Engine,
    input: CreativeInput,
) -> Result<CognitiveCreativeReport, Error> {
    engine.dispatch_creative(route(), input)
}

pub fn run_laws(
    engine: &Engine,
    registry: &LawRegistry,
    problem: &LawProblem,
) -> Result<SourcedLawReport, LawError> {
    engine.dispatch_laws(route(), registry, problem)
}
