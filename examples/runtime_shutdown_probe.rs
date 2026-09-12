//! Isolated process acceptance fixture, never shipped as a product command.
use sarmg_server_runtime::{
    BoundListeners, HttpServer, LifecycleParticipant, ProcessSignals, ProductDescriptor,
    ServerRuntime, ShutdownLimits, WorkScope,
};
use std::{sync::Arc, time::Duration};

struct Participant(WorkScope, std::sync::mpsc::Sender<()>);

impl Drop for Participant {
    fn drop(&mut self) {
        eprintln!("STATE_OWNER_DROPPED");
    }
}

#[async_trait::async_trait]
impl LifecycleParticipant for Participant {
    fn quiesce(&self) {
        self.0.quiesce();
    }
    fn cancel_ordinary_work(&self) {
        self.0.cancel_ordinary_work();
    }
    fn active_tasks(&self) -> (usize, usize) {
        (self.0.work_tasks.len(), self.0.commit_tasks.len())
    }
    async fn drain_requests(&self) {
        self.0.drain_requests().await;
    }
    async fn drain_commits(&self) {
        self.0.drain_commits().await;
    }
    async fn close_state(&self) -> Result<(), String> {
        // A deadline with an unfinished commit must never reach this callback.
        eprintln!("STATE_CLOSED");
        let _ = self.1.send(());
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let signals = ProcessSignals::install()?;
    let listeners = BoundListeners::bind(["127.0.0.1:0".parse()?])?;
    let scope = WorkScope::new();
    let (release, blocked) = std::sync::mpsc::channel::<()>();
    let (started, ready) = tokio::sync::oneshot::channel();
    scope.commit_tasks.spawn(async move {
        tokio::task::spawn_blocking(move || {
            started.send(()).unwrap();
            blocked.recv().unwrap();
        })
        .await
        .unwrap();
    });
    ready.await?;
    let runtime = ServerRuntime::builder(ProductDescriptor {
        id: "dufs-shutdown-fixture".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        foundation_revision: "80c49f2f811d8d47dbbcdb6c9521924de7d3184e".into(),
        profile: "server-filesystem".into(),
        capabilities: vec![],
    })
    .build()
    .await?;
    let mut transport = HttpServer::new(listeners, signals);
    transport.shutdown_limits = ShutdownLimits {
        grace: Duration::from_millis(100),
        forced: Duration::from_millis(100),
    };
    transport.participant = Some(Arc::new(Participant(scope, release)));
    println!("READY");
    match runtime.serve(transport, axum::Router::new()).await {
        Err(sarmg_server_runtime::Error::ShutdownIncomplete(error)) => {
            eprintln!("INCOMPLETE {}", serde_json::to_string(&error.report)?);
            std::process::exit(1);
        }
        result => {
            eprintln!("UNEXPECTED {result:?}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fixture_close_callback_unblocks_only_after_normal_draining() {
        let (release, blocked) = std::sync::mpsc::channel();
        let participant = Participant(WorkScope::new(), release);
        assert_eq!(participant.active_tasks(), (0, 0));
        assert!(blocked.try_recv().is_err());
        participant.quiesce();
        participant.cancel_ordinary_work();
        participant.drain_requests().await;
        participant.drain_commits().await;
        assert!(blocked.try_recv().is_err());
        participant.close_state().await.unwrap();
        assert_eq!(blocked.try_recv(), Ok(()));
    }
}
