use fw_error::app_error::AppError;
use fw_error::{FwError, FwResult};
use serde::Serialize;
use serde::de::DeserializeOwned;

#[inline]
pub fn parse_json<T: DeserializeOwned>(json_str: &str) -> Result<T, AppError> {
    serde_json::from_str::<T>(json_str).map_err(|e| AppError::ParseError("json", e.to_string()))
}

#[inline]
pub fn fmt_json<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string(value).map_err(|e| AppError::FormatError("json", e.to_string()))
}

#[inline]
pub fn fmt_json_as_u8<T: Serialize>(value: &T) -> Result<Vec<u8>, AppError> {
    serde_json::to_vec(value).map_err(|e| AppError::FormatError("json", e.to_string()))
}

#[inline]
pub fn parse_yaml<T: DeserializeOwned>(yaml_str: &str) -> Result<T, AppError> {
    serde_yaml::from_str::<T>(yaml_str).map_err(|e| AppError::ParseError("yaml", e.to_string()))
}

#[inline]
pub fn parse_json_in_fw<T: DeserializeOwned>(json_str: &str) -> FwResult<T> {
    serde_json::from_str::<T>(json_str).map_err(|e| FwError::ParseError(e.to_string()))
}

#[inline]
pub fn parse_yaml_in_fw<T: DeserializeOwned>(yaml_str: &str) -> FwResult<T> {
    serde_yaml::from_str::<T>(yaml_str).map_err(|e| FwError::ParseError(e.to_string()))
}

#[inline]
pub fn parse_msgpack<T: DeserializeOwned>(values: &Vec<u8>) -> Result<T, AppError> {
    rmp_serde::from_slice::<T>(values).map_err(|e| AppError::ParseError("msgpack", e.to_string()))
}

#[inline]
pub fn fmt_msgpack<T: Serialize>(value: &T) -> Result<Vec<u8>, AppError> {
    rmp_serde::to_vec(value).map_err(|e| AppError::FormatError("msgpack", e.to_string()))
}
