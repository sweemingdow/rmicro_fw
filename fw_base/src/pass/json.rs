use crate::WebPassContext;
use crate::pass::gw_pass::AuthInfoPassStrategy;
use fw_crypto::b64;
use fw_error::{FwError, FwResult};

pub struct JsonPassStrategy;

impl AuthInfoPassStrategy for JsonPassStrategy {
    fn encode(&self, ctx: &WebPassContext) -> FwResult<Vec<u8>> {
        let bytes =
            serde_json::to_vec(ctx).map_err(|e| FwError::SerializeError("json", e.to_string()))?;

        Ok(b64::encode_for_url(&bytes).into_bytes())
    }

    fn decode(&self, values: &Vec<u8>) -> FwResult<WebPassContext> {
        let decoded_bytes = b64::decode_for_url(values)?;

        let ctx = serde_json::from_slice::<WebPassContext>(&decoded_bytes)
            .map_err(|e| FwError::DeserializeError("json", e.to_string()))?;

        Ok(ctx)
    }
}
