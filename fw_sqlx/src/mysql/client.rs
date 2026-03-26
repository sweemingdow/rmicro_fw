use fw_error::{FwError, FwResult};
use std::time;

#[derive(Debug)]
pub struct MySqlOptions {
    pub addr: String,

    pub username: String,

    pub password: String,

    pub db_name: String,

    pub max_conn: Option<u16>,

    pub min_conn: Option<u16>,

    pub max_lifetime: Option<time::Duration>,

    pub idle_timeout: Option<time::Duration>,
}

pub async fn init_mysql(ops: MySqlOptions) -> FwResult<sqlx::Pool<sqlx::MySql>> {
    let min_conn = ops.min_conn.unwrap_or(2);
    let max_conn = ops.max_conn.unwrap_or(16);
    let max_lifetime = ops
        .max_lifetime
        .unwrap_or(time::Duration::from_secs(60 * 30));
    let idle_timeout = ops.idle_timeout.unwrap_or(time::Duration::from_secs(120));

    sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(max_conn as u32)
        .min_connections(min_conn as u32)
        .max_lifetime(max_lifetime)
        .idle_timeout(idle_timeout)
        .connect(
            format!(
                "mysql://{}:{}@{}/{}",
                ops.username, ops.password, ops.addr, ops.db_name
            )
            .as_str(),
        )
        .await
        .map_err(|e| FwError::InitError("mysql", format!("with options, ops={:?}, err={}", ops, e)))
}
