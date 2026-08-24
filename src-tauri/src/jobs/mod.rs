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
    pub fn update(
        &mut self,
        id: Uuid,
        status: JobStatus,
        progress: f32,
        error_message: Option<String>,
    ) -> Option<GenerationJob> {
        self.jobs.get_mut(&id).map(|job| {
            job.status = status;
            job.progress = progress.clamp(0.0, 100.0);
            job.error_message = error_message;
            job.clone()
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
    pub fn stop_all(&mut self) -> Vec<GenerationJob> {
        self.jobs
            .values_mut()
            .filter_map(|job| {
                let active = matches!(job.status, JobStatus::Queued | JobStatus::Running);
                if active {
                    job.status = JobStatus::Stopped;
                    Some(job.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{GenerationJob, JobKind};
    use chrono::Utc;

    fn job() -> GenerationJob {
        GenerationJob {
            id: Uuid::new_v4(),
            gateway_profile_id: "mock-default".into(),
            kind: JobKind::Video,
            operation: Some("generate".into()),
            model_id: Some("mock-video".into()),
            status: JobStatus::Queued,
            progress: 0.0,
            request_json: serde_json::json!({"prompt":"demo"}),
            error_message: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn cancel_marks_job_canceled() {
        let mut manager = JobManager::default();
        let created = manager.insert(job());
        let canceled = manager.cancel(created.id).expect("job should exist");
        assert!(matches!(canceled.status, JobStatus::Canceled));
    }

    #[test]
    fn retry_creates_new_queued_job() {
        let mut manager = JobManager::default();
        let created = manager.insert(job());
        let retried = manager.retry(created.id).expect("job should exist");
        assert_ne!(created.id, retried.id);
        assert!(matches!(retried.status, JobStatus::Queued));
    }

    #[test]
    fn stop_all_marks_only_active_jobs_stopped() {
        let mut manager = JobManager::default();
        let created = manager.insert(job());
        let stopped = manager.stop_all();
        assert_eq!(stopped.len(), 1);
        assert!(matches!(
            manager.get(created.id).unwrap().status,
            JobStatus::Stopped
        ));
    }

    #[test]
    fn update_persists_progress_and_error_state() {
        let mut manager = JobManager::default();
        let created = manager.insert(job());
        let updated = manager
            .update(
                created.id,
                JobStatus::Failed,
                128.0,
                Some("REMOTE_JOB_FAILED".into()),
            )
            .expect("job should exist");
        assert!(matches!(updated.status, JobStatus::Failed));
        assert_eq!(updated.progress, 100.0);
        assert_eq!(updated.error_message.as_deref(), Some("REMOTE_JOB_FAILED"));
    }
}
