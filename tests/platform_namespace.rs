#[path = "support/fixtures.rs"]
mod fixtures;

use fixtures::{Error, TestServer, server, with_new_upload_headers};
use reqwest::Method;
use rstest::rstest;

#[rstest]
fn physical_aliases_cannot_create_or_remove_platform_namespace(
    server: TestServer,
) -> Result<(), Error> {
    std::fs::create_dir_all(server.path().join("api/v2"))?;
    std::os::unix::fs::symlink("api", server.path().join("shortcut"))?;
    let forbidden = server.url().join("shortcut/v2/auth")?;
    let upload = with_new_upload_headers(server.request(Method::PUT, forbidden), 3)
        .body("bad")
        .send()?;
    assert_eq!(upload.status(), 404);
    assert!(!server.path().join("api/v2/auth").exists());

    let mkdir = server
        .request(Method::POST, server.url().join("__dufs__/api/mkdir")?)
        .header("content-type", "application/json")
        .header("x-dufs-operation-id", uuid::Uuid::new_v4().to_string())
        .body(r#"{"path":"/shortcut/v2/auth"}"#)
        .send()?;
    assert_eq!(mkdir.status(), 400);
    assert!(!server.path().join("api/v2/auth").exists());

    let deletion = server
        .request(Method::DELETE, server.url().join("shortcut/v2")?)
        .header("x-dufs-operation-id", uuid::Uuid::new_v4().to_string())
        .send()?;
    assert!(deletion.status().is_client_error());
    assert!(server.path().join("api/v2").is_dir());

    let ordinary = with_new_upload_headers(
        server.request(Method::PUT, server.url().join("api/notes.txt")?),
        4,
    )
    .body("safe")
    .send()?;
    assert_eq!(ordinary.status(), 201);
    assert_eq!(
        std::fs::read_to_string(server.path().join("api/notes.txt"))?,
        "safe"
    );
    assert!(server.path().join("shortcut").is_symlink());
    Ok(())
}
