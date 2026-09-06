use axum::body::Body;
use bytes::Bytes;
use http::{HeaderMap, header::CONTENT_TYPE};

pub fn body_full(content: impl Into<Bytes>) -> Body {
    Body::from(content.into())
}

/// Match one syntactically valid Content-Type field by its media-type essence.
/// Parameters are allowed, but duplicate fields and malformed parameters fail
/// closed instead of being hidden by a `split(';').next()` prefix check.
pub(crate) fn request_content_type_is(headers: &HeaderMap, expected: &str) -> bool {
    let mut values = headers.get_all(CONTENT_TYPE).iter();
    let (Some(value), None) = (values.next(), values.next()) else {
        return false;
    };
    value
        .to_str()
        .ok()
        .and_then(|value| value.parse::<mime_guess::mime::Mime>().ok())
        .is_some_and(|value| value.essence_str().eq_ignore_ascii_case(expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::header::HeaderValue;

    #[test]
    fn content_type_matching_validates_the_complete_single_field() {
        let mut headers = HeaderMap::new();
        for (value, expected) in [
            ("application/json", true),
            ("Application/JSON; charset=utf-8", true),
            ("application/json; charset=\"utf-8\"", true),
            ("text/plain", false),
            ("application/json;garbage", false),
            ("application/json; charset", false),
            ("application/json;=utf-8", false),
        ] {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static(value));
            assert_eq!(
                request_content_type_is(&headers, "application/json"),
                expected,
                "unexpected result for {value}"
            );
        }

        headers.append(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        assert!(!request_content_type_is(&headers, "application/json"));
    }
}
