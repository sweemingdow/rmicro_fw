use anyhow::anyhow;
use fw_error::{AppError, FwResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::task_local;

task_local! {
    static CURRENT_CONTEXT:WebContext
}

pub async fn into_scope<F, T>(ctx: WebContext, f: F) -> T
where
    F: Future<Output = T>,
{
    CURRENT_CONTEXT.scope(ctx, f).await
}

pub fn from_scope() -> Result<WebContext, AppError> {
    CURRENT_CONTEXT
        .try_get()
        .map_err(|e| AppError::InternalError(anyhow!(e)))
}

#[derive(Debug, Clone)]
pub struct WebContext(Arc<WebContextInner>);

#[derive(Debug, Serialize, Deserialize)]
pub struct WebContextInner {
    req_id: String,

    uid: Option<String>,

    client_type: u8,

    client_version: String,

    in_white: bool,

    in_callback: bool,

    in_open: bool, // more fields...
}

impl WebContext {
    pub fn new(inner: WebContextInner) -> Self {
        Self(Arc::new(inner))
    }

    // 提供便捷的访问方法
    pub fn req_id(&self) -> &str {
        &self.0.req_id
    }

    pub fn __no_matter_uid(&self) -> &str {
        self.0.uid.as_deref().unwrap_or("")
    }

    pub fn uid_with_check(&self) -> Result<&str, AppError> {
        let uid = self.__no_matter_uid();

        if !self.0.in_white && !self.0.in_callback && !self.0.in_open {
            if uid == "" {
                return Err(AppError::UnauthorizedError());
            }
        }

        Ok(uid)
    }

    pub fn client_type(&self) -> u8 {
        self.0.client_type
    }

    pub fn client_version(&self) -> &str {
        &self.0.client_version
    }
}
