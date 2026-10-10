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
    assert!(markup.contains("/index.js") && markup.contains("/dist/platform.css"));
    assert!(markup.contains("id=\"xczs-root\"") && markup.contains("id=\"index-data\""));

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

    let native = parse(
        server.get(
            server
                .url()
                .join(&format!("__xczs__/api/list?path=/docs&all={tag_id}"))?,
        )?,
    )?;
    assert_eq!(native["paths"][0]["name"], "original.txt");
    assert_eq!(native["file_tags"][0]["file_id"], old_id);
    assert_eq!(native["file_tags"][0]["tags"][0]["name"], "重要");
    let resolved = parse(server.get(endpoint(&server, "file?path=docs/original.txt")?)?)?;
    assert_eq!(resolved["file_id"], old_id);

    // A replacement before the next scan must not display or match old tags.
    std::fs::write(
        server.path().join("docs/original.txt"),
        b"temporary replacement",
    )?;
    let changed = parse(
        server.get(
            server
                .url()
                .join(&format!("__xczs__/api/list?path=/docs&all={tag_id}"))?,
        )?,
    )?;
    assert!(
        changed["paths"]
            .as_array()
            .ok_or("paths missing")?
            .is_empty()
    );
    let untagged = parse(
        server.get(
            server
                .url()
                .join(&format!("__xczs__/api/list?path=/docs&exclude={tag_id}"))?,
        )?,
    )?;
    assert_eq!(untagged["paths"][0]["name"], "original.txt");
    assert!(untagged["file_tags"][0]["file_id"].is_null());
    // Restore the original test fixture and refresh its identity explicitly.
    std::fs::write(server.path().join("docs/original.txt"), b"unchanged")?;

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

#[rstest]
fn directory_tags_filter_files_preserve_operations_and_bind_cursors(
    server: TestServer,
) -> Result<(), Error> {
    std::fs::create_dir_all(server.path().join("tag-filter-case/child"))?;
    for name in ["a.txt", "b.txt", "none.txt", "child/nested.txt"] {
        std::fs::write(server.path().join("tag-filter-case").join(name), name)?;
    }
    assert_eq!(
        server
            .request(Method::POST, endpoint(&server, "scan")?)
            .send()?
            .status(),
        StatusCode::OK
    );
    let mut tag_ids = Vec::new();
    for name in ["alpha", "beta"] {
        tag_ids.push(
            parse(send_json(
                server.request(Method::POST, endpoint(&server, "tags")?),
                json!({"name":name,"color":null}),
            )?)?["id"]
                .as_i64()
                .ok_or("tag id missing")?,
        );
    }
    for (name, tags) in [
        ("a.txt", vec![tag_ids[0]]),
        ("b.txt", tag_ids.clone()),
        ("child/nested.txt", vec![tag_ids[0]]),
    ] {
        let data = parse(server.get(endpoint(
            &server,
            &format!("file?path=tag-filter-case/{name}"),
        )?)?)?;
        let id = data["file_id"].as_i64().ok_or("file id missing")?;
        assert_eq!(
            send_json(
                server.request(Method::POST, endpoint(&server, "file-tags")?),
                json!({"file_ids":[id],"tag_ids":tags,"action":"add"})
            )?
            .status(),
            StatusCode::NO_CONTENT
        );
    }
    let list = |query: &str| -> Result<Value, Error> {
        parse(
            server.get(
                server
                    .url()
                    .join(&format!("__xczs__/api/list?path=/tag-filter-case&{query}"))?,
            )?,
        )
    };
    let both = list(&format!("all={},{}", tag_ids[0], tag_ids[1]))?;
    assert_eq!(both["paths"][0]["name"], "b.txt");
    assert_eq!(both["paths"].as_array().ok_or("paths missing")?.len(), 1);
    assert!(
        both["paths"][0]["revision"]
            .as_str()
            .is_some_and(|value| value.len() == 64)
    );
    assert_eq!(
        both["file_tags"][0]["tags"]
            .as_array()
            .ok_or("tags missing")?
            .len(),
        2
    );
    assert_eq!(
        list(&format!("any={}", tag_ids[1]))?["paths"][0]["name"],
        "b.txt"
    );
    assert_eq!(
        list(&format!("exclude={}", tag_ids[0]))?["paths"][0]["name"],
        "none.txt"
    );
    let recursive = list(&format!("q=.txt&all={}", tag_ids[0]))?;
    assert_eq!(
        recursive["paths"].as_array().ok_or("paths missing")?.len(),
        3
    );
    let first = list(&format!("all={}&limit=1", tag_ids[0]))?;
    let cursor = first["next_cursor"].as_str().ok_or("cursor missing")?;
    assert_eq!(
        list(&format!("all={}&limit=1&cursor={cursor}", tag_ids[0]))?["paths"][0]["name"],
        "b.txt"
    );
    let changed = server.get(server.url().join(&format!(
        "__xczs__/api/list?path=/tag-filter-case&all={},{}&limit=1&cursor={cursor}",
        tag_ids[0], tag_ids[1]
    ))?)?;
    assert_eq!(changed.status(), StatusCode::BAD_REQUEST);
    assert_eq!(parse(changed)?["code"], "invalid_list_cursor");
    let invalid = server.get(
        server
            .url()
            .join("__xczs__/api/list?path=/tag-filter-case&all=-1")?,
    )?;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(parse(invalid)?["code"], "invalid_tag_filter");
    std::fs::write(server.path().join("tag-filter-case/unindexed.txt"), b"new")?;
    let excluded = list(&format!("exclude={}", tag_ids[0]))?;
    assert_eq!(
        excluded["paths"].as_array().ok_or("paths missing")?.len(),
        2
    );
    let invalid_path = server.get(endpoint(&server, "file?path=../outside")?)?;
    assert_eq!(invalid_path.status(), StatusCode::BAD_REQUEST);
    Ok(())
}
