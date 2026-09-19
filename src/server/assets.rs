use super::{Response, Server, status_not_found};
use crate::http_utils::body_full;

use crate::utils::encode_hex;
use bytes::Bytes;
use headers::{ContentLength, HeaderMapExt};
use http::{
    StatusCode,
    header::{CACHE_CONTROL, HeaderValue},
};
use sha2::{Digest, Sha256};

struct EmbeddedAsset {
    name: &'static str,
    contents: &'static [u8],
    content_type: &'static str,
}

// 浏览器客户端的源码统一位于 clients/web，并在编译期完整嵌入二进制；
// 运行时 URL 仍由下面的资源名和内容摘要生成，与仓库目录名解耦。
const PLATFORM_ASSETS: &[EmbeddedAsset] = include!(concat!(env!("OUT_DIR"), "/platform-assets.rs"));
const EMBEDDED_ASSETS: &[EmbeddedAsset] = include!(concat!(env!("OUT_DIR"), "/web-assets.rs"));
pub(super) fn embedded_assets_prefix() -> String {
    let mut digest = Sha256::new();
    for asset in EMBEDDED_ASSETS.iter().chain(PLATFORM_ASSETS) {
        digest.update((asset.name.len() as u64).to_be_bytes());
        digest.update(asset.name.as_bytes());
        digest.update((asset.content_type.len() as u64).to_be_bytes());
        digest.update(asset.content_type.as_bytes());
        digest.update((asset.contents.len() as u64).to_be_bytes());
        digest.update(asset.contents);
    }
    format!("__dufs_assets_{}/", encode_hex(digest.finalize()))
}

impl Server {
    pub(super) fn handle_internal(
        &self,
        req_path: &str,
        head_only: bool,
        res: &mut Response,
    ) -> bool {
        let Some(name) = req_path.strip_prefix(&self.content.assets_prefix) else {
            return false;
        };
        let Some(asset) = embedded_asset(name) else {
            status_not_found(res);
            res.headers_mut().insert(
                "x-content-type-options",
                HeaderValue::from_static("nosniff"),
            );
            return true;
        };

        res.headers_mut()
            .typed_insert(ContentLength(asset.contents.len() as u64));
        if !head_only {
            *res.body_mut() = body_full(Bytes::from_static(asset.contents));
        }
        res.headers_mut()
            .insert("content-type", HeaderValue::from_static(asset.content_type));
        debug_assert_eq!(res.status(), StatusCode::OK);
        res.headers_mut().insert(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
        res.headers_mut().insert(
            "x-content-type-options",
            HeaderValue::from_static("nosniff"),
        );
        true
    }

    pub(super) fn is_public_asset_path(&self, req_path: &str) -> bool {
        req_path
            .strip_prefix(&self.content.assets_prefix)
            .is_some_and(|name| embedded_asset(name).is_some())
    }
}

fn embedded_asset(name: &str) -> Option<&'static EmbeddedAsset> {
    EMBEDDED_ASSETS
        .iter()
        .chain(PLATFORM_ASSETS)
        .find(|asset| asset.name == name)
}
