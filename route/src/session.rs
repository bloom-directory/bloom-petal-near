use crate::{
    api_types::{QuoteResponse, TokenResponse},
    assets::ResolvedOrigin,
    evm::PreparedTransaction,
    input::NewSwapRequest,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct History {
    pub at_ms: u64,
    pub from: String,
    pub to: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FailedSession {
    pub schema_version: u32,
    pub id: String,
    pub wallet: String,
    pub created_ms: u64,
    pub updated_ms: u64,
    pub state: String,
    pub last_error: String,
}

pub fn failure_key(wallet: &str, id: &str) -> String {
    format!("swaps/{wallet}/{id}/failure.json")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub schema_version: u32,
    pub id: String,
    pub wallet: String,
    pub wallet_address: String,
    pub created_ms: u64,
    pub updated_ms: u64,
    pub state: String,
    pub request: NewSwapRequest,
    pub origin_token: TokenResponse,
    pub origin: ResolvedOrigin,
    pub quote: QuoteResponse,
    pub quote_hash: String,
    pub quote_verified: bool,
    pub prepared_transaction: Option<PreparedTransaction>,
    pub prepared_digest: Option<String>,
    pub plan_md: Option<String>,
    pub staging_started: bool,
    pub outbox_id: Option<String>,
    pub outbox_state: Option<String>,
    pub origin_tx_hash: Option<String>,
    pub approval: Option<serde_json::Value>,
    pub deposit_submit_state: Option<String>,
    pub upstream_status: Option<String>,
    pub swap_details: Option<serde_json::Value>,
    pub last_error: Option<String>,
    pub history: Vec<History>,
}

impl Session {
    pub fn transition(&mut self, now: u64, next: &str, reason: &str) {
        let from = std::mem::replace(&mut self.state, next.into());
        self.updated_ms = now;
        self.history.push(History {
            at_ms: now,
            from,
            to: next.into(),
            reason: reason.chars().take(256).collect(),
        });
        if self.history.len() > 100 {
            self.history.remove(0);
        }
    }
    pub fn key(&self) -> String {
        format!("swaps/{}/{}/session.json", self.wallet, self.id)
    }
    pub fn terminal(&self) -> bool {
        matches!(
            self.state.as_str(),
            "settled_success" | "settled_refunded" | "settled_failed" | "abandoned"
        )
    }
}

pub fn key(wallet: &str, id: &str) -> String {
    format!("swaps/{wallet}/{id}/session.json")
}
