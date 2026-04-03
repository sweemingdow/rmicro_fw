use crate::axum_srv::utils;
use anyhow::anyhow;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use fw_adapter::web_bridge::wrapper::AnyErrorWrapper;
use fw_base::constants::web_const::{AUTH_INFO_KEY, GW_DISPATCH_KEY};
use fw_base::context::web::WebContext;
use fw_base::pass::gw_pass::{AuthInfoPassStrategy, AuthInfoPassStrategyEnum};
use fw_base::{WebPassContext, get_gw_dispatch_val, parse_json, web_ctx_into_scope, get_pass_strategy};
use fw_error::app_error::AppError;


pub async fn auth_layer(req: Request, next: Next) -> Response {
    let headers = req.headers();

    if let Some(resp) = is_from_gw_forwarded(headers) {
        return resp;
    };

    // 解析鉴权参数
    match parse_auth_info(headers) {
        Ok(ctx) => web_ctx_into_scope(WebContext::new(ctx), next.run(req)).await,
        Err(ae) => AnyErrorWrapper::from_app_err(ae).into_response(),
    }
}

// 验证是否从网关转发而来
fn is_from_gw_forwarded(headers: &HeaderMap<HeaderValue>) -> Option<Response> {
    let Some(dispatch_val) = utils::get_val_from_header(GW_DISPATCH_KEY, headers) else {
        return Some(
            AnyErrorWrapper(anyhow!(AppError::RejectError(
                "not coming from gw dispatch".to_string()
            )))
            .into_response(),
        );
    };

    let expected_dispatch = match get_gw_dispatch_val() {
        Ok(val) => val,
        Err(ae) => {
            return Some(AnyErrorWrapper::from_app_err(ae).into_response());
        }
    };

    if dispatch_val != expected_dispatch {
        return Some(
            AnyErrorWrapper(anyhow!(AppError::RejectError(format!(
                "invalid dispatch val, expected={}, got={}",
                expected_dispatch, dispatch_val
            ))))
            .into_response(),
        );
    }

    None
}

fn parse_auth_info(headers: &HeaderMap<HeaderValue>) -> Result<WebPassContext, AppError> {
    let Some(val) = utils::get_val_from_header(AUTH_INFO_KEY, headers) else {
        return Err(AppError::ForbiddenError("no authed info found".to_string()));
    };

    match get_pass_strategy().decode(&val.as_bytes().to_vec()) {
        Ok(ctx) => Ok(ctx),
        Err(fe) => Err(AppError::from(fe)),
    }
}
