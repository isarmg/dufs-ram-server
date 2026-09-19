#[path = "support/fixtures.rs"]
mod fixtures;

use dufs::utils::encode_hex;
use fixtures::{Error, TestServer, server, with_new_upload_headers};
use reqwest::header::{CACHE_CONTROL, CONTENT_TYPE};
use reqwest::{Method, StatusCode};
use rstest::rstest;
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const LONG_LIVED_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";
const PRIVATE_NO_STORE: &str = "private, no-store";

#[rstest]
fn embedded_assets_are_content_addressed_and_cacheable(server: TestServer) -> Result<(), Error> {
    verify_embedded_assets(&server)
}

fn verify_embedded_assets(server: &TestServer) -> Result<(), Error> {
    let page = server.get(server.url())?.error_for_status()?.text()?;
    let index_js = extract_asset_url(&page, "src", "index.js")?;
    let index_css = extract_asset_url(&page, "href", "index.css")?;
    let favicon = extract_asset_url(&page, "href", "favicon.ico")?;

    let asset_prefix = shared_asset_prefix([&index_js, &index_css, &favicon])?;
    let advertised_digest = asset_prefix
        .strip_prefix("/__dufs_assets_")
        .and_then(|value| value.strip_suffix('/'))
        .ok_or("Embedded asset prefix has an unexpected form")?;

    let web_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("clients/web");
    let mut web_files = [
        "login.js",
        "index.js",
        "index.css",
        "login.css",
        "favicon.ico",
    ]
    .into_iter()
    .map(|name| web_root.join(name))
    .collect::<Vec<_>>();
    collect_javascript_files(&web_root.join("modules"), &mut web_files)?;
    web_files.sort();
    let mut assets = Vec::with_capacity(web_files.len());
    for path in web_files {
        let name = path
            .strip_prefix(&web_root)?
            .to_str()
            .ok_or("Web asset path is not UTF-8")?
            .replace('\\', "/");
        let content_type = match path.extension().and_then(|value| value.to_str()) {
            Some("js") => "application/javascript; charset=UTF-8",
            Some("css") => "text/css; charset=UTF-8",
            Some("ico") => "image/x-icon",
            _ => panic!("unexpected Web runtime asset"),
        };
        let served_path = match name.as_str() {
            "index.js" => index_js.clone(),
            "index.css" => index_css.clone(),
            "favicon.ico" => favicon.clone(),
            _ => format!("{asset_prefix}{name}"),
        };
        assets.push((name, served_path, content_type));
    }
    let mut platform = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/clients/web/dist"))?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<Result<Vec<_>, _>>()?;
    platform.sort();
    for path in platform {
        let name = path.file_name().unwrap().to_str().unwrap();
        if name == "platform.d.ts" {
            continue;
        }
        let content_type = match path.extension().and_then(|value| value.to_str()) {
            Some("js") => "application/javascript; charset=UTF-8",
            Some("css") => "text/css; charset=UTF-8",
            Some("woff2") => "font/woff2",
            Some("svg") => {
                let hash = name
                    .strip_prefix("foundation-icon-")
                    .and_then(|value| value.strip_suffix(".svg"))
                    .expect("only compiled Foundation icons are runtime SVG assets");
                assert_eq!(hash.len(), 64);
                assert_eq!(hash, encode_hex(Sha256::digest(std::fs::read(&path)?)));
                "image/svg+xml"
            }
            Some("txt") => "text/plain; charset=UTF-8",
            _ => panic!("unexpected runtime platform asset"),
        };
        assets.push((
            format!("dist/{name}"),
            format!("{asset_prefix}dist/{name}"),
            content_type,
        ));
    }
    let mut digest = Sha256::new();
    for (name, path, expected_content_type) in assets {
        let response = server.get(server.url().join(&path)?)?;
        assert_eq!(response.status(), StatusCode::OK, "asset={path}");
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some(expected_content_type),
            "asset={path}"
        );
        assert_eq!(
            response
                .headers()
                .get(CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some(LONG_LIVED_CACHE_CONTROL),
            "asset={path}"
        );

        let contents = response.bytes()?;
        digest.update((name.len() as u64).to_be_bytes());
        digest.update(name.as_bytes());
        digest.update((expected_content_type.len() as u64).to_be_bytes());
        digest.update(expected_content_type.as_bytes());
        digest.update((contents.len() as u64).to_be_bytes());
        digest.update(&contents);
    }
    assert_eq!(
        encode_hex(digest.finalize()),
        advertised_digest,
        "Embedded asset prefix must be the SHA-256 digest of the served assets"
    );

    let missing_path = format!("{asset_prefix}missing.js");
    let missing = server.get(server.url().join(&missing_path)?)?;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        missing
            .headers()
            .get(CACHE_CONTROL)
            .and_then(|value| value.to_str().ok()),
        Some(PRIVATE_NO_STORE)
    );
    assert_eq!(missing.text()?, "Not Found");

    let reserved_component = asset_prefix
        .strip_prefix('/')
        .and_then(|value| value.strip_suffix('/'))
        .ok_or("Embedded asset URL has an invalid reserved component")?;
    let upload = with_new_upload_headers(
        server.request(
            Method::PUT,
            server.url().join(&format!("{asset_prefix}user-file.txt"))?,
        ),
        7,
    )
    .body("blocked")
    .send()?;
    assert_eq!(upload.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(upload.headers()["allow"], "GET, HEAD");

    let mkdir = server
        .request(Method::POST, server.url().join("__dufs__/api/mkdir")?)
        .header(CONTENT_TYPE, "application/json")
        .header("X-Dufs-Operation-Id", Uuid::new_v4().to_string())
        .body(json!({"path": format!("/{reserved_component}/directory")}).to_string())
        .send()?;
    assert_eq!(mkdir.status(), StatusCode::BAD_REQUEST);
    assert!(!server.path().join(reserved_component).exists());
    Ok(())
}

fn collect_javascript_files(
    directory: &std::path::Path,
    files: &mut Vec<std::path::PathBuf>,
) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        assert!(!metadata.file_type().is_symlink());
        if metadata.is_dir() {
            collect_javascript_files(&path, files)?;
        } else {
            assert_eq!(
                path.extension().and_then(|value| value.to_str()),
                Some("js")
            );
            files.push(path);
        }
    }
    Ok(())
}

fn extract_asset_url(page: &str, attribute: &str, filename: &str) -> Result<String, Error> {
    let marker = format!(r#"{attribute}=""#);
    let expected_suffix = format!("/{filename}");
    let mut found = None;

    for (start, _) in page.match_indices(&marker) {
        let value = &page[start + marker.len()..];
        let Some(end) = value.find('"') else {
            return Err(format!("The {attribute} attribute is missing its closing quote").into());
        };
        let value = &value[..end];
        if !value.ends_with(&expected_suffix) {
            continue;
        }
        if found.replace(value.to_string()).is_some() {
            return Err(format!("Directory page contains duplicate {filename} URLs").into());
        }
    }

    let path = found.ok_or_else(|| format!("Directory page is missing the {filename} URL"))?;
    if !path.starts_with('/') {
        return Err(format!("Embedded asset URL is not absolute: {path}").into());
    }
    Ok(path)
}

fn shared_asset_prefix(paths: [&str; 3]) -> Result<String, Error> {
    let filenames = ["index.js", "index.css", "favicon.ico"];
    let mut shared_prefix = None;

    for (path, filename) in paths.into_iter().zip(filenames) {
        let prefix = path
            .strip_suffix(filename)
            .ok_or_else(|| format!("Embedded asset URL does not end with {filename}: {path}"))?;
        if let Some(shared_prefix) = shared_prefix {
            if shared_prefix != prefix {
                return Err("Directory page assets do not share one content digest prefix".into());
            }
        } else {
            shared_prefix = Some(prefix);
        }
    }

    let shared_prefix = shared_prefix.ok_or("Directory page does not contain embedded assets")?;
    let digest = shared_prefix
        .strip_prefix("/__dufs_assets_")
        .and_then(|value| value.strip_suffix('/'))
        .ok_or_else(|| format!("Unexpected embedded asset prefix: {shared_prefix}"))?;
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(format!("Embedded asset digest is not lowercase SHA-256: {digest}").into());
    }

    Ok(shared_prefix.to_string())
}
