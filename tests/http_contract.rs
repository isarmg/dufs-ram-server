#[path = "support/fixtures.rs"]
mod fixtures;

use fixtures::{Error, TEST_PASSWORD, TEST_USER, TestServer, server};
use rstest::rstest;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::TcpStream,
    path::Path,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    method: String,
    path: String,
    authenticated: bool,
    headers: BTreeMap<String, String>,
    body: String,
    preset: BTreeMap<String, String>,
    status: u16,
    required_headers: BTreeMap<String, String>,
    forbidden_headers: Vec<String>,
    body_contains: String,
    response_fields: BTreeMap<String, serde_json::Value>,
    file_changes: BTreeMap<String, String>,
    operation_state: Option<String>,
}

fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &path, files);
            } else if entry.file_type().unwrap().is_file() {
                files.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

#[rstest]
fn recorded_http_contract_uses_real_sockets(server: TestServer) -> Result<(), Error> {
    let session = server.login(TEST_USER, TEST_PASSWORD)?;
    for source in [
        include_str!("fixtures/http-contract/routes.json"),
        include_str!("fixtures/http-contract/authentication.json"),
        include_str!("fixtures/http-contract/path-cases.json"),
        include_str!("fixtures/http-contract/download-cases.json"),
        include_str!("fixtures/http-contract/upload-cases.json"),
        include_str!("fixtures/http-contract/mutation-cases.json"),
    ] {
        for case in serde_json::from_str::<Vec<Case>>(source)? {
            for (path, content) in &case.preset {
                std::fs::write(server.path().join(path), content)?;
            }
            let mut expected = snapshot(server.path());
            for (path, content) in &case.file_changes {
                expected.insert(path.clone(), content.as_bytes().to_vec());
            }
            let mut wire = format!(
                "{} {} HTTP/1.1\r\nHost: localhost:{}\r\nConnection: close\r\nContent-Length: {}\r\n",
                case.method,
                case.path,
                server.url().port().unwrap(),
                case.body.len()
            );
            if case.authenticated {
                wire.push_str(&format!("Cookie: {}\r\nOrigin: {}\r\nSec-Fetch-Site: same-origin\r\nX-CSRF-Token: {}\r\n", session.cookie(), server.url().origin().ascii_serialization(), session.csrf_token()));
            }
            for (name, value) in &case.headers {
                wire.push_str(&format!("{name}: {value}\r\n"));
            }
            wire.push_str("\r\n");
            wire.push_str(&case.body);
            let mut socket = TcpStream::connect(("127.0.0.1", server.url().port().unwrap()))?;
            socket.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
            socket.write_all(wire.as_bytes())?;
            let mut bytes = Vec::new();
            socket.read_to_end(&mut bytes)?;
            let text = String::from_utf8(bytes)?;
            let (head, body) = text.split_once("\r\n\r\n").ok_or("missing HTTP head")?;
            assert_eq!(
                head.split_whitespace().nth(1).unwrap().parse::<u16>()?,
                case.status,
                "{}: {text}",
                case.id
            );
            let headers: BTreeMap<_, _> = head
                .lines()
                .skip(1)
                .filter_map(|line| line.split_once(':'))
                .map(|(key, value)| (key.to_ascii_lowercase(), value.trim().to_owned()))
                .collect();
            for (key, value) in &case.required_headers {
                assert_eq!(headers.get(key), Some(value), "{}: {head}", case.id);
            }
            for key in &case.forbidden_headers {
                assert!(!headers.contains_key(key), "{}: {head}", case.id);
            }
            assert_eq!(
                headers.get("x-dufs-operation-state"),
                case.operation_state.as_ref(),
                "{}",
                case.id
            );
            assert!(body.contains(&case.body_contains), "{}: {body}", case.id);
            if case.method == "HEAD" {
                assert!(body.is_empty(), "{}", case.id);
            }
            if !case.response_fields.is_empty() {
                let json: serde_json::Value = serde_json::from_str(body)?;
                for (key, value) in &case.response_fields {
                    assert_eq!(&json[key], value, "{}", case.id);
                }
            }
            assert_eq!(
                snapshot(server.path()),
                expected,
                "{} filesystem effects",
                case.id
            );
        }
    }
    Ok(())
}
