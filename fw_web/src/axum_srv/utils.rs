use axum::http::{HeaderMap, HeaderValue};

pub fn get_val_from_header<'a>(key: &str, header: &'a HeaderMap<HeaderValue>) -> Option<&'a str> {
    header.get(key).and_then(|v| v.to_str().ok())
}

pub fn get_bytes_from_header<'a>(
    key: &str,
    header: &'a HeaderMap<HeaderValue>,
) -> Option<&'a [u8]> {
    header.get(key).and_then(|v| Some(v.as_bytes()))
}
