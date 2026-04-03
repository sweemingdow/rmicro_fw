use crate::WebPassContext;
use crate::pass::gw_pass::AuthInfoPassStrategy;
use fw_crypto::b64;
use fw_error::{FwError, FwResult};

// 直接用竖线分割版本
pub struct DirectlyPassStrategy;

impl AuthInfoPassStrategy for DirectlyPassStrategy {
    fn encode(&self, ctx: &WebPassContext) -> FwResult<Vec<u8>> {

       todo!()
    }

    fn decode(&self, values: &Vec<u8>) -> FwResult<WebPassContext> {
        todo!()
    }
}
