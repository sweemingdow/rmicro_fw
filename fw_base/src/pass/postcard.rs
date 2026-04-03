use crate::WebPassContext;
use crate::pass::gw_pass::AuthInfoPassStrategy;
use fw_crypto::b64;
use fw_error::{FwError, FwResult};

pub struct PostcardPassStrategy;

impl AuthInfoPassStrategy for PostcardPassStrategy {
    fn encode(&self, ctx: &WebPassContext) -> FwResult<Vec<u8>> {
        let bytes = postcard::to_stdvec(&ctx)
            .map_err(|e| FwError::SerializeError("postcard", e.to_string()))?;

        Ok(b64::encode_for_url(&bytes).into_bytes())
    }

    fn decode(&self, values: &Vec<u8>) -> FwResult<WebPassContext> {
        let decoded_bytes = b64::decode_for_url(values)?;

        let ctx = postcard::from_bytes::<WebPassContext>(&decoded_bytes)
            .map_err(|e| FwError::DeserializeError("postcard", e.to_string()))?;

        Ok(ctx)
    }
}
