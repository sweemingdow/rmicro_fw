pub mod runner;
pub mod utils;

pub use utils as my_utils;

pub mod configuration;
pub mod constants;
pub mod context;
pub mod lazy;
pub mod pass;

pub use crate::lazy::gw_lazy::{
    get_gw_dispatch_val, get_pass_strategy, init_pass_strategy, set_gw_dispatch_val,
};

pub use crate::context::web::from_scope as web_ctx_from_scope;
pub use crate::context::web::into_scope as web_ctx_into_scope;

pub use crate::utils::parser::{
    fmt_json, fmt_json_as_u8, parse_json, parse_json_in_fw, parse_yaml, parse_yaml_in_fw,
};

pub type WebPassContext = context::web::WebContextInner;
