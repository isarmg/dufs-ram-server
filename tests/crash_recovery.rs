#[path = "support/fixtures.rs"]
mod fixtures;

use fixtures::{Error, TEST_ACCOUNT, server, with_resume_upload_headers, with_upload_headers};
use reqwest::Method;
use uuid::Uuid;

// Run this same real-process contract against the extracted release binary
// using DUFS_TEST_BINARY. SIGABRT exercises process death without unwind;
// this does not claim Foundation catch_unwind can isolate release panic.
#[test]
fn abnormal_exit_preserves_current_upload_checkpoint_and_committed_result() -> Result<(), Error> {
    let mut server = server(&[] as &[&str], &[TEST_ACCOUNT]);
    let id = Uuid::new_v4();
    let target = "abort-recovery.bin";
    let partial = with_upload_headers(
        server.request(Method::PUT, server.url().join(target)?),
        id,
        6,
    )
    .body("abc")
    .send()?;
    assert_eq!(partial.status(), 409);
    assert_eq!(partial.headers()["x-dufs-upload-offset"], "3");
    assert!(!server.path().join(target).exists());
    server.abort_process_and_wait();
    server.restart_with_default_auth();
    let status = server
        .request(Method::HEAD, server.url().join(target)?)
        .header("x-dufs-upload-id", id.to_string())
        .send()?;
    assert_eq!(status.status(), 200);
    assert_eq!(status.headers()["x-dufs-upload-offset"], "3");
    assert_eq!(status.headers()["x-dufs-operation-state"], "running");
    let complete = with_resume_upload_headers(
        server.request(Method::PATCH, server.url().join(target)?),
        id,
        6,
        3,
    )
    .body("def")
    .send()?;
    assert_eq!(complete.status(), 204);
    assert_eq!(std::fs::read(server.path().join(target))?, b"abcdef");
    server.abort_process_and_wait();
    server.restart_with_default_auth();
    let status = server
        .request(Method::HEAD, server.url().join(target)?)
        .header("x-dufs-upload-id", id.to_string())
        .send()?;
    assert_eq!(status.status(), 200);
    assert_eq!(status.headers()["x-dufs-upload-offset"], "6");
    assert_eq!(status.headers()["x-dufs-operation-state"], "committed");
    assert_eq!(std::fs::read(server.path().join(target))?, b"abcdef");
    Ok(())
}
