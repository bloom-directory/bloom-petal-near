pub mod api;
pub mod api_types;
pub mod assets;
pub mod evm;
pub mod input;
pub mod quote_signature;
pub mod redaction;
pub mod render;
pub mod session;
pub mod settings;
pub mod workflow;

pub fn session_view(ctx: &petal::Ctx, field: &str) -> petal::DispatchResponse {
    let wallet = match petal::param(ctx, "wallet") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let id = match petal::param(ctx, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    workflow::session_route(wallet, id, field)
}

pub mod prelude {
    pub use crate::workflow::*;
    pub use petal::*;
}
