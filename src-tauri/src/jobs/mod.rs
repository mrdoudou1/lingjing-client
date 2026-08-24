use crate::domain::{GenerationJob, JobStatus};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Default)]
pub struct JobManager {
    jobs: HashMap<Uuid, GenerationJob>,
}
impl JobManager {
    pub fn insert(&mut self, job: GenerationJob) -> GenerationJob {
        self.jobs.insert(job.id, job.clone());
        job
    }
    pub fn get(&self, id: Uuid) -> Option<GenerationJob> {
        self.jobs.get(&id).cloned()
    }
    pub fn cancel(&mut self, id: Uuid) -> Option<GenerationJob> {
        self.jobs.get_mut(&id).map(|job| {
            job.status = JobStatus::Canceled;
            job.clone()
        })
    }
    pub fn retry(&mut self, id: Uuid) -> Option<GenerationJob> {
        self.jobs.get(&id).cloned().map(|mut job| {
            job.id = Uuid::new_v4();
            job.status = JobStatus::Queued;
            job.progress = 0.0;
            self.jobs.insert(job.id, job.clone());
            job
        })
    }
    pub fn list(&self, kind: Option<&str>, status: Option<&str>) -> Vec<GenerationJob> {
        self.jobs
            .values()
            .filter(|job| {
                let kind_matches =
                    kind.is_none_or(|value| format!("{:?}", job.kind).eq_ignore_ascii_case(value));
                let status_matches = status
                    .is_none_or(|value| format!("{:?}", job.status).eq_ignore_ascii_case(value));
                kind_matches && status_matches
            })
            .cloned()
            .collect()
    }
}
