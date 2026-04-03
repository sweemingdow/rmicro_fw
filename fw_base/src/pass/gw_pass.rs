use crate::WebPassContext;
use crate::pass::json::JsonPassStrategy;
use crate::pass::msgpack::MsgpackPassStrategy;
use crate::pass::postcard::PostcardPassStrategy;
use fw_error::FwResult;

pub trait AuthInfoPassStrategy {
    fn encode(&self, ctx: &WebPassContext) -> FwResult<Vec<u8>>;

    fn decode(&self, values: &Vec<u8>) -> FwResult<WebPassContext>;
}

pub enum AuthInfoPassStrategyEnum {
    Postcard(PostcardPassStrategy),
    Json(JsonPassStrategy),
    Msgpack(MsgpackPassStrategy),
}

impl AuthInfoPassStrategyEnum {
    // 工厂方法改为枚举的构造函数
    pub fn new(strategy: &str) -> Self {
        match strategy {
            "postcard" => AuthInfoPassStrategyEnum::Postcard(PostcardPassStrategy),
            "json" => AuthInfoPassStrategyEnum::Json(JsonPassStrategy),
            "msgpack" => AuthInfoPassStrategyEnum::Msgpack(MsgpackPassStrategy),
            _ => AuthInfoPassStrategyEnum::Json(JsonPassStrategy),
        }
    }
}

impl AuthInfoPassStrategy for AuthInfoPassStrategyEnum {
    fn encode(&self, ctx: &WebPassContext) -> FwResult<Vec<u8>> {
        match self {
            AuthInfoPassStrategyEnum::Postcard(s) => s.encode(ctx),
            AuthInfoPassStrategyEnum::Json(s) => s.encode(ctx),
            AuthInfoPassStrategyEnum::Msgpack(s) => s.encode(ctx),
        }
    }

    fn decode(&self, values: &Vec<u8>) -> FwResult<WebPassContext> {
        match self {
            AuthInfoPassStrategyEnum::Postcard(s) => s.decode(values),
            AuthInfoPassStrategyEnum::Json(s) => s.decode(values),
            AuthInfoPassStrategyEnum::Msgpack(s) => s.decode(values),
        }
    }
}
