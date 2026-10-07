//! Small executable fixture for the custom Rust-orchestrator training world.
//!
//! Decisions and state reduction are separate, all messages are typed, queues
//! are bounded, effect authority is explicit, and duplicate submissions are
//! idempotent. This is a curriculum fixture, not the Lightcone runtime.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct JobId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Capability {
    Submit,
    Execute,
    Resolve,
    Cancel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Submit {
        idempotency_key: String,
        payload: String,
    },
    StartNext,
    Complete {
        job_id: JobId,
        result: String,
    },
    Fail {
        job_id: JobId,
        error: String,
        retryable: bool,
    },
    Cancel {
        job_id: JobId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Accepted {
        job_id: JobId,
        idempotency_key: String,
        payload: String,
    },
    Duplicate {
        job_id: JobId,
        idempotency_key: String,
    },
    Started {
        job_id: JobId,
        attempt: u32,
    },
    Completed {
        job_id: JobId,
        result: String,
    },
    Failed {
        job_id: JobId,
        error: String,
        retryable: bool,
    },
    Requeued {
        job_id: JobId,
    },
    Cancelled {
        job_id: JobId,
    },
    Rejected {
        command: &'static str,
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Completed(String),
    Failed(String),
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    pub id: JobId,
    pub idempotency_key: String,
    pub payload: String,
    pub attempts: u32,
    pub status: JobStatus,
}

#[derive(Clone, Debug)]
pub struct Orchestrator {
    max_queued: usize,
    max_attempts: u32,
    next_job: u64,
    jobs: BTreeMap<JobId, Job>,
    by_key: BTreeMap<String, JobId>,
    queue: VecDeque<JobId>,
    running: Option<JobId>,
    history: Vec<Event>,
}

impl Orchestrator {
    pub fn new(max_queued: usize, max_attempts: u32) -> Result<Self, String> {
        if max_queued == 0 || max_attempts == 0 {
            return Err("queue and attempt limits must be positive".into());
        }
        Ok(Self {
            max_queued,
            max_attempts,
            next_job: 1,
            jobs: BTreeMap::new(),
            by_key: BTreeMap::new(),
            queue: VecDeque::new(),
            running: None,
            history: Vec::new(),
        })
    }

    pub fn handle(
        &mut self,
        capabilities: &BTreeSet<Capability>,
        command: Command,
    ) -> Result<Vec<Event>, String> {
        let events = self.decide(capabilities, command);
        for event in &events {
            self.apply(event.clone())?;
        }
        self.verify()?;
        Ok(events)
    }

    pub fn job(&self, id: JobId) -> Option<&Job> {
        self.jobs.get(&id)
    }

    pub fn history(&self) -> &[Event] {
        &self.history
    }

    pub fn queued(&self) -> usize {
        self.queue.len()
    }

    pub fn running(&self) -> Option<JobId> {
        self.running
    }

    fn decide(&self, capabilities: &BTreeSet<Capability>, command: Command) -> Vec<Event> {
        let (required, name) = match &command {
            Command::Submit { .. } => (Capability::Submit, "submit"),
            Command::StartNext => (Capability::Execute, "start_next"),
            Command::Complete { .. } | Command::Fail { .. } => (Capability::Resolve, "resolve"),
            Command::Cancel { .. } => (Capability::Cancel, "cancel"),
        };
        if !capabilities.contains(&required) {
            return vec![Event::Rejected {
                command: name,
                reason: format!("missing capability {required:?}"),
            }];
        }
        match command {
            Command::Submit {
                idempotency_key,
                payload,
            } => {
                if let Some(job_id) = self.by_key.get(&idempotency_key) {
                    return vec![Event::Duplicate {
                        job_id: *job_id,
                        idempotency_key,
                    }];
                }
                if self.queue.len() >= self.max_queued {
                    return vec![Event::Rejected {
                        command: "submit",
                        reason: "bounded queue is full".into(),
                    }];
                }
                vec![Event::Accepted {
                    job_id: JobId(self.next_job),
                    idempotency_key,
                    payload,
                }]
            }
            Command::StartNext => match (self.running, self.queue.front()) {
                (Some(_), _) => vec![Event::Rejected {
                    command: "start_next",
                    reason: "a worker is already running".into(),
                }],
                (None, Some(job_id)) => {
                    let attempts = self.jobs.get(job_id).map_or(1, |job| job.attempts + 1);
                    vec![Event::Started {
                        job_id: *job_id,
                        attempt: attempts,
                    }]
                }
                (None, None) => vec![Event::Rejected {
                    command: "start_next",
                    reason: "queue is empty".into(),
                }],
            },
            Command::Complete { job_id, result } => {
                if self.running == Some(job_id) {
                    vec![Event::Completed { job_id, result }]
                } else {
                    vec![Event::Rejected {
                        command: "complete",
                        reason: "job is not the running job".into(),
                    }]
                }
            }
            Command::Fail {
                job_id,
                error,
                retryable,
            } => {
                let Some(job) = self.jobs.get(&job_id) else {
                    return vec![Event::Rejected {
                        command: "fail",
                        reason: "unknown job".into(),
                    }];
                };
                if self.running != Some(job_id) {
                    return vec![Event::Rejected {
                        command: "fail",
                        reason: "job is not the running job".into(),
                    }];
                }
                let retry = retryable && job.attempts < self.max_attempts;
                let mut events = vec![Event::Failed {
                    job_id,
                    error,
                    retryable: retry,
                }];
                if retry {
                    events.push(Event::Requeued { job_id });
                }
                events
            }
            Command::Cancel { job_id } => match self.jobs.get(&job_id) {
                Some(job) if matches!(job.status, JobStatus::Queued | JobStatus::Running) => {
                    vec![Event::Cancelled { job_id }]
                }
                Some(_) => vec![Event::Rejected {
                    command: "cancel",
                    reason: "job is already terminal".into(),
                }],
                None => vec![Event::Rejected {
                    command: "cancel",
                    reason: "unknown job".into(),
                }],
            },
        }
    }

    fn apply(&mut self, event: Event) -> Result<(), String> {
        match &event {
            Event::Accepted {
                job_id,
                idempotency_key,
                payload,
            } => {
                if self.jobs.contains_key(job_id) || self.by_key.contains_key(idempotency_key) {
                    return Err("accepted event violates job identity uniqueness".into());
                }
                self.next_job = self.next_job.max(job_id.0 + 1);
                self.by_key.insert(idempotency_key.clone(), *job_id);
                self.queue.push_back(*job_id);
                self.jobs.insert(
                    *job_id,
                    Job {
                        id: *job_id,
                        idempotency_key: idempotency_key.clone(),
                        payload: payload.clone(),
                        attempts: 0,
                        status: JobStatus::Queued,
                    },
                );
            }
            Event::Duplicate { .. } | Event::Rejected { .. } => {}
            Event::Started { job_id, attempt } => {
                if self.running.is_some() || self.queue.pop_front() != Some(*job_id) {
                    return Err("started event violates queue ownership".into());
                }
                let job = self.jobs.get_mut(job_id).ok_or("started unknown job")?;
                job.attempts = *attempt;
                job.status = JobStatus::Running;
                self.running = Some(*job_id);
            }
            Event::Completed { job_id, result } => {
                if self.running != Some(*job_id) {
                    return Err("completed event does not own the running slot".into());
                }
                self.jobs
                    .get_mut(job_id)
                    .ok_or("completed unknown job")?
                    .status = JobStatus::Completed(result.clone());
                self.running = None;
            }
            Event::Failed { job_id, error, .. } => {
                if self.running != Some(*job_id) {
                    return Err("failed event does not own the running slot".into());
                }
                self.jobs
                    .get_mut(job_id)
                    .ok_or("failed unknown job")?
                    .status = JobStatus::Failed(error.clone());
                self.running = None;
            }
            Event::Requeued { job_id } => {
                let job = self.jobs.get_mut(job_id).ok_or("requeued unknown job")?;
                if !matches!(job.status, JobStatus::Failed(_)) {
                    return Err("only a failed job can be requeued".into());
                }
                job.status = JobStatus::Queued;
                self.queue.push_back(*job_id);
            }
            Event::Cancelled { job_id } => {
                let job = self.jobs.get_mut(job_id).ok_or("cancelled unknown job")?;
                job.status = JobStatus::Cancelled;
                self.queue.retain(|queued| queued != job_id);
                if self.running == Some(*job_id) {
                    self.running = None;
                }
            }
        }
        self.history.push(event);
        Ok(())
    }

    fn verify(&self) -> Result<(), String> {
        if self.queue.len() > self.max_queued {
            return Err("bounded queue exceeded its capacity".into());
        }
        let queued = self.queue.iter().copied().collect::<BTreeSet<_>>();
        if queued.len() != self.queue.len() {
            return Err("queue contains a duplicate job".into());
        }
        for (id, job) in &self.jobs {
            let in_queue = queued.contains(id);
            let is_running = self.running == Some(*id);
            match job.status {
                JobStatus::Queued if !in_queue || is_running => {
                    return Err("queued job ownership is inconsistent".into());
                }
                JobStatus::Running if in_queue || !is_running => {
                    return Err("running job ownership is inconsistent".into());
                }
                JobStatus::Completed(_) | JobStatus::Failed(_) | JobStatus::Cancelled
                    if in_queue || is_running =>
                {
                    return Err("terminal job still owns live execution state".into());
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(values: &[Capability]) -> BTreeSet<Capability> {
        values.iter().copied().collect()
    }

    #[test]
    fn duplicate_submission_is_idempotent() {
        let mut orchestrator = Orchestrator::new(2, 2).unwrap();
        let capability = caps(&[Capability::Submit]);
        let first = orchestrator
            .handle(
                &capability,
                Command::Submit {
                    idempotency_key: "same".into(),
                    payload: "one".into(),
                },
            )
            .unwrap();
        let second = orchestrator
            .handle(
                &capability,
                Command::Submit {
                    idempotency_key: "same".into(),
                    payload: "different".into(),
                },
            )
            .unwrap();
        assert!(matches!(
            first.as_slice(),
            [Event::Accepted {
                job_id: JobId(1),
                ..
            }]
        ));
        assert!(matches!(
            second.as_slice(),
            [Event::Duplicate {
                job_id: JobId(1),
                ..
            }]
        ));
        assert_eq!(orchestrator.queued(), 1);
    }

    #[test]
    fn bounded_queue_rejects_overload() {
        let mut orchestrator = Orchestrator::new(1, 2).unwrap();
        let capability = caps(&[Capability::Submit]);
        for key in ["one", "two"] {
            let _ = orchestrator
                .handle(
                    &capability,
                    Command::Submit {
                        idempotency_key: key.into(),
                        payload: key.into(),
                    },
                )
                .unwrap();
        }
        assert_eq!(orchestrator.queued(), 1);
        assert!(
            matches!(orchestrator.history().last(), Some(Event::Rejected { reason, .. }) if reason.contains("full"))
        );
    }

    #[test]
    fn capability_does_not_expand() {
        let mut orchestrator = Orchestrator::new(1, 1).unwrap();
        let events = orchestrator
            .handle(&BTreeSet::new(), Command::StartNext)
            .unwrap();
        assert!(
            matches!(events.as_slice(), [Event::Rejected { reason, .. }] if reason.contains("Execute"))
        );
    }

    #[test]
    fn lifecycle_is_typed_and_owned() {
        let mut orchestrator = Orchestrator::new(2, 2).unwrap();
        orchestrator
            .handle(
                &caps(&[Capability::Submit]),
                Command::Submit {
                    idempotency_key: "job".into(),
                    payload: "work".into(),
                },
            )
            .unwrap();
        orchestrator
            .handle(&caps(&[Capability::Execute]), Command::StartNext)
            .unwrap();
        assert_eq!(orchestrator.running(), Some(JobId(1)));
        orchestrator
            .handle(
                &caps(&[Capability::Resolve]),
                Command::Complete {
                    job_id: JobId(1),
                    result: "done".into(),
                },
            )
            .unwrap();
        assert_eq!(
            orchestrator.job(JobId(1)).map(|job| &job.status),
            Some(&JobStatus::Completed("done".into()))
        );
    }

    #[test]
    fn retry_is_bounded_and_requires_failure() {
        let mut orchestrator = Orchestrator::new(2, 2).unwrap();
        orchestrator
            .handle(
                &caps(&[Capability::Submit]),
                Command::Submit {
                    idempotency_key: "job".into(),
                    payload: "work".into(),
                },
            )
            .unwrap();
        for expected_retry in [true, false] {
            orchestrator
                .handle(&caps(&[Capability::Execute]), Command::StartNext)
                .unwrap();
            let events = orchestrator
                .handle(
                    &caps(&[Capability::Resolve]),
                    Command::Fail {
                        job_id: JobId(1),
                        error: "temporary".into(),
                        retryable: true,
                    },
                )
                .unwrap();
            assert_eq!(
                events
                    .iter()
                    .any(|event| matches!(event, Event::Requeued { .. })),
                expected_retry
            );
        }
        assert!(matches!(
            orchestrator.job(JobId(1)).unwrap().status,
            JobStatus::Failed(_)
        ));
    }

    #[test]
    fn cancellation_releases_live_ownership() {
        let mut orchestrator = Orchestrator::new(2, 1).unwrap();
        orchestrator
            .handle(
                &caps(&[Capability::Submit]),
                Command::Submit {
                    idempotency_key: "job".into(),
                    payload: "work".into(),
                },
            )
            .unwrap();
        orchestrator
            .handle(&caps(&[Capability::Execute]), Command::StartNext)
            .unwrap();
        orchestrator
            .handle(
                &caps(&[Capability::Cancel]),
                Command::Cancel { job_id: JobId(1) },
            )
            .unwrap();
        assert_eq!(orchestrator.running(), None);
        assert_eq!(
            orchestrator.job(JobId(1)).unwrap().status,
            JobStatus::Cancelled
        );
    }
}
