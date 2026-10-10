//! Request-scoped cancellation shared by search, answers and indexing.
use crate::store::Result;
use std::{
    future::Future,
    sync::{Arc, Mutex},
};
use tokio::sync::{watch, OwnedSemaphorePermit, Semaphore};

pub const CANCELLED: &str = "Operation cancelled.";

struct Job {
    id: String,
    running: bool,
    cancel: watch::Sender<bool>,
}

#[derive(Default)]
struct Registry {
    closed: bool,
    job: Option<Job>,
}

#[derive(Clone, Default)]
pub struct OperationJobs {
    registry: Arc<Mutex<Registry>>,
}

impl OperationJobs {
    /// Registration happens before the UI sends the operation command, so a
    /// cancellation arriving ahead of that command still prevents execution.
    /// Pending registrations hold no operation permit and may be superseded.
    pub fn begin(&self) -> Result<String> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| "Operation state is unavailable.")?;
        if registry.closed {
            return Err("Cachella is closing.".into());
        }
        if registry.job.as_ref().is_some_and(|job| job.running) {
            return Err("An operation is already running.".into());
        }
        let id = uuid::Uuid::new_v4().to_string();
        let (cancel, _) = watch::channel(false);
        registry.job = Some(Job {
            id: id.clone(),
            running: false,
            cancel,
        });
        Ok(id)
    }

    pub fn cancel(&self, id: &str) -> Result<bool> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| "Operation state is unavailable.")?;
        let Some(job) = registry.job.as_ref().filter(|job| job.id == id) else {
            return Ok(false);
        };
        job.cancel.send_replace(true);
        if !job.running {
            registry.job = None;
        }
        Ok(true)
    }

    pub fn shutdown(&self) {
        if let Ok(mut registry) = self.registry.lock() {
            registry.closed = true;
            if let Some(job) = registry.job.as_ref() {
                job.cancel.send_replace(true);
                if !job.running {
                    registry.job = None;
                }
            }
        }
    }

    pub async fn run<T>(
        &self,
        id: &str,
        gate: Arc<Semaphore>,
        future: impl Future<Output = Result<T>>,
    ) -> Result<T> {
        let (permit, mut cancel) = {
            let mut registry = self
                .registry
                .lock()
                .map_err(|_| "Operation state is unavailable.")?;
            let job = registry
                .job
                .as_mut()
                .filter(|job| job.id == id && !job.running)
                .ok_or(CANCELLED)?;
            // Validate identity and acquire the permit in one registry critical
            // section. A stale command must never borrow a valid retry's slot.
            let permit = gate
                .try_acquire_owned()
                .map_err(|_| "Wait for the current model operation to finish.")?;
            job.running = true;
            (permit, job.cancel.subscribe())
        };
        let active = ActiveOperation {
            jobs: self.clone(),
            id: id.into(),
            _permit: permit,
        };
        let result = tokio::select! {
            biased;
            _ = cancelled(&mut cancel) => Err(CANCELLED.into()),
            result = future => result,
        };
        // Serialize completion with cancel: an accepted cancellation overrides
        // a simultaneously ready result before it crosses the IPC boundary.
        active.finish(result)
    }
}

async fn cancelled(receiver: &mut watch::Receiver<bool>) {
    while !*receiver.borrow_and_update() {
        if receiver.changed().await.is_err() {
            return;
        }
    }
}

struct ActiveOperation {
    jobs: OperationJobs,
    id: String,
    _permit: OwnedSemaphorePermit,
}

impl ActiveOperation {
    fn finish<T>(&self, result: Result<T>) -> Result<T> {
        let mut registry = self
            .jobs
            .registry
            .lock()
            .map_err(|_| "Operation state is unavailable.")?;
        let was_cancelled = registry
            .job
            .as_ref()
            .filter(|job| job.id == self.id)
            .map(|job| *job.cancel.borrow())
            .unwrap_or(true);
        if registry.job.as_ref().is_some_and(|job| job.id == self.id) {
            registry.job = None;
        }
        if was_cancelled {
            Err(CANCELLED.into())
        } else {
            result
        }
    }
}

impl Drop for ActiveOperation {
    fn drop(&mut self) {
        if let Ok(mut registry) = self.jobs.registry.lock() {
            if registry.job.as_ref().is_some_and(|job| job.id == self.id) {
                registry.job = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn stale_registration_is_rejected_before_consulting_a_busy_operation_gate() {
        let jobs = OperationJobs::default();
        let gate = Arc::new(Semaphore::new(1));
        let stale = jobs.begin().unwrap();
        assert!(jobs.cancel(&stale).unwrap());
        let current = jobs.begin().unwrap();
        let (ready, started) = tokio::sync::oneshot::channel();
        let owner = jobs.clone();
        let key = current.clone();
        let shared_gate = gate.clone();
        let task = tokio::spawn(async move {
            owner
                .run(&key, shared_gate, async {
                    ready.send(()).unwrap();
                    std::future::pending::<Result<()>>().await
                })
                .await
        });
        started.await.unwrap();
        assert_eq!(
            jobs.run(&stale, gate.clone(), async { Ok(()) })
                .await
                .unwrap_err(),
            CANCELLED
        );
        assert!(jobs.cancel(&current).unwrap());
        assert_eq!(task.await.unwrap().unwrap_err(), CANCELLED);
        assert_eq!(gate.available_permits(), 1);
    }

    #[tokio::test]
    async fn cancelled_pending_request_never_executes_and_does_not_cancel_retry() {
        let jobs = OperationJobs::default();
        let gate = Arc::new(Semaphore::new(1));
        let old = jobs.begin().unwrap();
        assert!(jobs.cancel(&old).unwrap());
        let next = jobs.begin().unwrap();
        assert!(!jobs.cancel(&old).unwrap());
        assert_eq!(
            jobs.run(&old, gate.clone(), async {
                panic!("cancelled work ran");
                #[allow(unreachable_code)]
                Ok(())
            })
            .await
            .unwrap_err(),
            CANCELLED
        );
        assert_eq!(
            jobs.run(&next, gate.clone(), async { Ok(42) })
                .await
                .unwrap(),
            42
        );
        assert_eq!(gate.available_permits(), 1);
    }

    #[tokio::test]
    async fn running_cancellation_drops_work_releases_gate_and_permits_retry() {
        struct OnDrop(Arc<AtomicBool>);
        impl Drop for OnDrop {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }
        let jobs = OperationJobs::default();
        let gate = Arc::new(Semaphore::new(1));
        let id = jobs.begin().unwrap();
        let dropped = Arc::new(AtomicBool::new(false));
        let (ready, started) = tokio::sync::oneshot::channel();
        let owner = jobs.clone();
        let key = id.clone();
        let shared_gate = gate.clone();
        let marker = dropped.clone();
        let task = tokio::spawn(async move {
            owner
                .run(&key, shared_gate, async move {
                    let _drop = OnDrop(marker);
                    let _ = ready.send(());
                    std::future::pending::<Result<()>>().await
                })
                .await
        });
        started.await.unwrap();
        assert_eq!(gate.available_permits(), 0);
        assert!(jobs.begin().is_err());
        assert!(!jobs.cancel("stale-request").unwrap());
        assert!(jobs.cancel(&id).unwrap());
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), task)
                .await
                .unwrap()
                .unwrap()
                .unwrap_err(),
            CANCELLED
        );
        assert!(dropped.load(Ordering::Acquire));
        assert_eq!(gate.available_permits(), 1);
        let retry = jobs.begin().unwrap();
        assert_eq!(jobs.run(&retry, gate, async { Ok(7) }).await.unwrap(), 7);
    }

    #[tokio::test]
    async fn shutdown_cancels_registered_work_and_refuses_later_registration() {
        let jobs = OperationJobs::default();
        let id = jobs.begin().unwrap();
        jobs.shutdown();
        assert!(jobs.begin().is_err());
        assert_eq!(
            jobs.run(&id, Arc::new(Semaphore::new(1)), async { Ok(()) })
                .await
                .unwrap_err(),
            CANCELLED
        );
    }
    #[tokio::test]
    async fn pending_registration_holds_no_permit_and_new_registration_supersedes_it() {
        let jobs = OperationJobs::default();
        let gate = Arc::new(Semaphore::new(1));
        let old = jobs.begin().unwrap();
        let current = jobs.begin().unwrap();
        assert_eq!(gate.available_permits(), 1);
        assert_eq!(
            jobs.run(&old, gate.clone(), async { Ok(1) })
                .await
                .unwrap_err(),
            CANCELLED
        );
        assert_eq!(jobs.run(&current, gate, async { Ok(2) }).await.unwrap(), 2);
        assert!(!jobs.cancel(&current).unwrap());
    }

    #[tokio::test]
    async fn aborting_the_command_releases_registration_and_operation_permit() {
        let jobs = OperationJobs::default();
        let gate = Arc::new(Semaphore::new(1));
        let id = jobs.begin().unwrap();
        let (ready, started) = tokio::sync::oneshot::channel();
        let owner = jobs.clone();
        let shared_gate = gate.clone();
        let task = tokio::spawn(async move {
            owner
                .run(&id, shared_gate, async move {
                    ready.send(()).unwrap();
                    std::future::pending::<Result<()>>().await
                })
                .await
        });
        started.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(gate.available_permits(), 1);
        let next = jobs.begin().unwrap();
        assert_eq!(
            jobs.run(&next, gate, async { Err::<(), _>("fixture error".into()) })
                .await
                .unwrap_err(),
            "fixture error"
        );
        assert!(!jobs.cancel(&next).unwrap());
    }

    #[tokio::test]
    async fn cancellation_closes_actual_inference_http_during_headers_or_response_body() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        for send_headers in [false, true] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let endpoint = format!("http://{}", listener.local_addr().unwrap());
            let (ready, started) = tokio::sync::oneshot::channel();
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut buffer = [0u8; 4096];
                assert!(socket.read(&mut buffer).await.unwrap() > 0);
                if send_headers {
                    socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1048576\r\nContent-Type: application/json\r\n\r\n{\"choices\":[").await.unwrap();
                }
                ready.send(()).unwrap();
                loop {
                    match socket.read(&mut buffer).await {
                        Ok(0) => break,
                        Ok(_) => (),
                        Err(error)
                            if matches!(
                                error.kind(),
                                std::io::ErrorKind::ConnectionReset
                                    | std::io::ErrorKind::BrokenPipe
                            ) =>
                        {
                            break
                        }
                        Err(error) => panic!("unexpected socket error: {error}"),
                    }
                }
            });
            let directory = tempfile::tempdir().unwrap();
            let mut store =
                crate::store::Store::open(&directory.path().join("test.sqlite")).unwrap();
            store
                .import_text("budget", "txt", "The budget is nine dollars.")
                .unwrap();
            let hits = store.search("budget").unwrap();
            let jobs = OperationJobs::default();
            let gate = Arc::new(Semaphore::new(1));
            let id = jobs.begin().unwrap();
            let key = id.clone();
            let owner = jobs.clone();
            let shared_gate = gate.clone();
            let task = tokio::spawn(async move {
                owner
                    .run(
                        &key,
                        shared_gate,
                        crate::inference::answer(&endpoint, "What is the budget?", hits, true),
                    )
                    .await
            });
            tokio::time::timeout(Duration::from_secs(2), started)
                .await
                .unwrap()
                .unwrap();
            assert!(jobs.cancel(&id).unwrap());
            assert_eq!(
                tokio::time::timeout(Duration::from_secs(1), task)
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap_err(),
                CANCELLED
            );
            tokio::time::timeout(Duration::from_secs(2), server)
                .await
                .expect("cancelled HTTP connection remained open")
                .unwrap();
            assert_eq!(gate.available_permits(), 1);
        }
    }
    #[tokio::test]
    async fn accepted_cancellation_wins_over_a_simultaneously_ready_result() {
        let jobs = OperationJobs::default();
        let gate = Arc::new(Semaphore::new(1));
        let id = jobs.begin().unwrap();
        let result = jobs
            .run(&id, gate.clone(), async {
                assert!(jobs.cancel(&id).unwrap());
                Ok("late answer")
            })
            .await;
        assert_eq!(result.unwrap_err(), CANCELLED);
        assert_eq!(gate.available_permits(), 1);
        assert!(!jobs.cancel(&id).unwrap());
    }
    #[tokio::test]
    async fn shutdown_notifies_a_running_answer_and_releases_its_gate() {
        let jobs = OperationJobs::default();
        let gate = Arc::new(Semaphore::new(1));
        let id = jobs.begin().unwrap();
        let (ready, started) = tokio::sync::oneshot::channel();
        let owner = jobs.clone();
        let shared_gate = gate.clone();
        let task = tokio::spawn(async move {
            owner
                .run(&id, shared_gate, async move {
                    ready.send(()).unwrap();
                    std::future::pending::<Result<()>>().await
                })
                .await
        });
        started.await.unwrap();
        jobs.shutdown();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), task)
                .await
                .unwrap()
                .unwrap()
                .unwrap_err(),
            CANCELLED
        );
        assert_eq!(gate.available_permits(), 1);
        assert!(jobs.begin().is_err());
    }
}
