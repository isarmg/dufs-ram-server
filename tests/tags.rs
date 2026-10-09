#[path = "support/fixtures.rs"]
mod fixtures;

use fixtures::{Error, TestServer, server};
use reqwest::blocking::{RequestBuilder, Response};
use reqwest::header::CONTENT_TYPE;
use reqwest::{Method, StatusCode};
use rstest::rstest;
use serde_json::{Value, json};

fn parse(response: Response) -> Result<Value, Error> {
    Ok(serde_json::from_str(&response.text()?)?)
}
fn send_json(request: RequestBuilder, value: Value) -> Result<Response, Error> {
    Ok(request
        .header(CONTENT_TYPE, "application/json")
        .body(serde_json::to_vec(&value)?)
        .send()?)
}

fn endpoint(server: &TestServer, path: &str) -> Result<reqwest::Url, Error> {
    Ok(server.url().join(&format!("api/v1/file-tags/{path}"))?)
}

#[rstest]
fn tags_survive_scans_and_relink_without_changing_file_contents(
    server: TestServer,
) -> Result<(), Error> {
    let anonymous = server
        .raw_request(Method::GET, endpoint(&server, "files")?)
        .send()?;
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
    let page = server.get(server.url().join("__xczs__/tags")?)?;
    assert_eq!(page.status(), StatusCode::OK);
    let markup = page.text()?;
    assert!(markup.contains("/dist/tags.js") && markup.contains("/dist/platform.css"));

    std::fs::create_dir(server.path().join("docs"))?;
    std::fs::write(server.path().join("docs/original.txt"), b"unchanged")?;
    let scan = server
        .request(Method::POST, endpoint(&server, "scan")?)
        .send()?;
    assert_eq!(scan.status(), StatusCode::OK);
    assert!(parse(scan)?["scanned"].as_u64().unwrap_or(0) >= 1);
    let files = parse(server.get(endpoint(&server, "files?scope=all")?)?)?;
    let old_id = files["files"]
        .as_array()
        .ok_or("file rows missing")?
        .iter()
        .find(|row| row["path"] == "docs/original.txt")
        .and_then(|row| row["id"].as_i64())
        .ok_or("file id missing")?;

    let tag = send_json(
        server.request(Method::POST, endpoint(&server, "tags")?),
        json!({"name": "重要", "color": "#2563eb"}),
    )?;
    assert_eq!(tag.status(), StatusCode::CREATED);
    let tag_id = parse(tag)?["id"].as_i64().ok_or("tag id missing")?;
    let add = send_json(
        server.request(Method::POST, endpoint(&server, "file-tags")?),
        json!({"file_ids": [old_id], "tag_ids": [tag_id], "action": "add"}),
    )?;
    assert_eq!(add.status(), StatusCode::NO_CONTENT);
    let filtered =
        parse(server.get(endpoint(&server, &format!("files?scope=all&all={tag_id}"))?)?)?;
    assert_eq!(filtered["total"], 1);

    std::fs::rename(
        server.path().join("docs/original.txt"),
        server.path().join("docs/renamed.txt"),
    )?;
    assert_eq!(
        server
            .request(Method::POST, endpoint(&server, "scan")?)
            .send()?
            .status(),
        StatusCode::OK
    );
    let all = parse(server.get(endpoint(&server, "files?scope=all&status=all")?)?)?;
    let rows = all["files"].as_array().ok_or("file rows missing")?;
    assert!(
        rows.iter()
            .any(|row| row["id"] == old_id && row["status"] == "missing")
    );
    let new_id = rows
        .iter()
        .find(|row| row["path"] == "docs/renamed.txt")
        .and_then(|row| row["id"].as_i64())
        .ok_or("renamed file missing")?;
    assert_eq!(
        send_json(
            server.request(
                Method::POST,
                endpoint(&server, &format!("files/{new_id}/relink"))?
            ),
            json!({"source_id": old_id})
        )?
        .status(),
        StatusCode::NO_CONTENT
    );
    let filtered =
        parse(server.get(endpoint(&server, &format!("files?scope=all&all={tag_id}"))?)?)?;
    assert_eq!(filtered["total"], 1);
    assert_eq!(filtered["files"][0]["path"], "docs/renamed.txt");
    let download = server.get(server.url().join("docs/renamed.txt")?)?;
    assert_eq!(download.status(), StatusCode::OK);
    assert_eq!(download.bytes()?.as_ref(), b"unchanged");
    let backup = server
        .request(Method::POST, endpoint(&server, "backup")?)
        .send()?;
    assert_eq!(backup.status(), StatusCode::OK);
    assert!(parse(backup)?["filename"].as_str().is_some());
    Ok(())
}
