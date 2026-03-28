use fw_error::app_error::AppError;
use serde::de::DeserializeOwned;

#[inline]
pub fn parse_json<T: DeserializeOwned>(json_str: &str) -> Result<T, AppError> {
    serde_json::from_str::<T>(json_str).map_err(|e| AppError::ParseError("json", e.to_string()))
}
