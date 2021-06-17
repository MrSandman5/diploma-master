use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Uint128, HumanAddr};

/// storage key for oracle state
pub static CONFIG_KEY: &[u8] = b"config";

/// block size
pub const BLOCK_SIZE: usize = 256;

/// Instantiation message
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct InitMsg {
    /// init user address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<HumanAddr>,
    /// init user history
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<History>
}

/// Handle message
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    AddHistory {
        user: HumanAddr,
        history: History
    },
}

/// Query message
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// get user history query
    GetHistory {
        /// user address
        user: HumanAddr,
    },
}

/// Query response
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueryResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// user history
    pub history: Option<History>,
    /// execution description
    pub message: String
}

/// Client credit history
#[derive(Serialize, Deserialize, Clone, JsonSchema, PartialEq, Debug)]
pub struct History {
    /// current funds
    pub debts: Uint128,
    /// all credits
    pub credits: Vec<Credit>,
}

/// Client credit data
#[derive(Serialize, Deserialize, Clone, JsonSchema, PartialEq, Debug)]
pub struct Credit {
    /// amount of money
    pub sum: Uint128,
    /// interest rate of credit
    pub interest_rate: Uint128,
    /// time to close credit (in months)
    pub time: Uint128,
    /// condition of closing
    pub is_closed: bool,
}
