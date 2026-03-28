use axum::http::{HeaderMap, HeaderValue};

pub fn get_val_from_header<'a>(key: &str, header: &'a HeaderMap<HeaderValue>) -> Option<&'a str> {
    header.get(key).and_then(|v| v.to_str().ok())
}

