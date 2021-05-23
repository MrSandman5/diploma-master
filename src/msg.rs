use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CosmosMsg, HumanAddr, Querier, StdResult, Uint128, Binary};

use secret_toolkit::snip20::{register_receive_msg, token_info_query, transfer_msg, TokenInfo};

/// storage key for auction state
pub const CONFIG_KEY: &[u8] = b"config";

/// block size
pub const BLOCK_SIZE: usize = 256;

/// Instantiation message
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    /// sell contract code hash and address
    pub sell_contract: ContractInfo,
    /// bid contract code hash and address
    pub bid_contract: ContractInfo,
    /// amount of tokens being sold
    pub credit_request: Vec<Credit>,
    /// Optional description of the auction
    #[serde(default)]
    pub description: Option<String>,
}

/// Handle messages
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// Receive gets called by the token contracts of the auction.  If it came from the sale token, it
    /// will consign the sent tokens.  If it came from the bid token, it will place a bid.  If any
    /// other address tries to call this, it will give an error message that the calling address is
    /// not a token in the auction.
    Receive {
        /// address of person or contract that sent the tokens that triggered this Receive
        sender: HumanAddr,
        /// address of the owner of the tokens sent to the auction
        from: HumanAddr,
        /// amount of tokens sent
        amount: Uint128,
        /// Optional base64 encoded message sent with the Send call -- not needed or used by this
        /// contract
        #[serde(default)]
        msg: Option<Binary>,
    },

    /// ViewBid will display the active bid made by the calling address
    ViewBid {},

    /// Finalize will close the auction
    Finalize {
        /// true if auction creator wants to keep the auction open if there are no active bids
        only_if_bids: bool,
    },
    /// If the auction holds any funds after it has closed (should never happen), this will return
    /// those funds to their owners.  Should never be needed, but included in case of unforeseen
    /// error
    ReturnAll {},
}

/// Queries
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Displays the auction information
    AuctionInfo {},
    CalculateProposal {
        proposal: Proposal,
    }
}

/// responses to queries
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    /// AuctionInfo query response
    AuctionInfo {
        /// sell token address and TokenInfo query response
        sell_token: Token,
        /// bid token address and TokenInfo query response
        bid_token: Token,
        /// amount of tokens being sold
        credit_request: Uint128,
        /// Optional description of auction
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// address of auction contract
        auction_address: HumanAddr,
        /// status of the auction can be "Accepting bids: Tokens to be sold have(not) been
        /// consigned" or "Closed" (will also state if there are outstanding funds after auction
        /// closure
        status: String,
        /// If the auction resulted in a swap, this will state the winning bid
        #[serde(skip_serializing_if = "Option::is_none")]
        winning_bid: Option<Uint128>,
    },
    CalculateProposal {
        /// amount of tokens to bid
        #[serde(skip_serializing_if = "Option::is_none")]
        credit_proposal: Option<Uint128>,
        /// execution description
        message: String,
    }
}

/// token's contract address and TokenInfo response
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct Token {
    /// contract address of token
    pub contract_address: HumanAddr,
    /// Tokeninfo query response
    pub token_info: TokenInfo,
}

/// success or failure response
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub enum ResponseStatus {
    Success,
    Failure,
}

/// Responses from handle functions
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    /// response from consign attempt
    Consign {
        /// success or failure
        status: ResponseStatus,
        /// execution description
        message: String,
        /// Optional amount consigned
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_consigned: Option<Uint128>,
        /// Optional amount that still needs to be consigned
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_needed: Option<Uint128>,
        /// Optional amount of tokens returned from escrow
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_returned: Option<Uint128>,
    },
    /// response from bid attempt
    Bid {
        /// success or failure
        status: ResponseStatus,
        /// execution description
        message: String,
        /// Optional amount of previous bid returned from escrow
        #[serde(skip_serializing_if = "Option::is_none")]
        previous_bid: Option<Uint128>,
        /// Optional amount bid
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_bid: Option<Uint128>,
        /// Optional amount of tokens returned from escrow
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_returned: Option<Uint128>,
    },
    /// response from closing the auction
    CloseAuction {
        /// success or failure
        status: ResponseStatus,
        /// execution description
        message: String,
        /// Optional amount of winning bid
        #[serde(skip_serializing_if = "Option::is_none")]
        winning_bid: Option<Uint128>,
        /// Optional amount of tokens returned form escrow
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_returned: Option<Uint128>,
    },
    /// generic status response
    Status {
        /// success or failure
        status: ResponseStatus,
        /// execution description
        message: String,
    },
}

/// client credit data
#[derive(Serialize, Deserialize, JsonSchema)]
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

/// credit proposition data
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Proposal {
    /// amount of money
    pub sum: Uint128,
    /// interest rate of credit
    pub interest_rate: Uint128,
    /// time to close credit (in months)
    pub time: Uint128,
}

/// code hash and address of a contract
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug)]
pub struct ContractInfo {
    /// contract's code hash string
    pub code_hash: String,
    /// contract's address
    pub address: HumanAddr,
}

impl ContractInfo {
    /// Returns a StdResult<CosmosMsg> used to execute Transfer
    ///
    /// # Arguments
    ///
    /// * `recipient` - address tokens are to be sent to
    /// * `amount` - Uint128 amount of tokens to send
    pub fn transfer_msg(&self, recipient: HumanAddr, amount: Uint128) -> StdResult<CosmosMsg> {
        transfer_msg(
            recipient,
            amount,
            None,
            BLOCK_SIZE,
            self.code_hash.clone(),
            self.address.clone(),
        )
    }

    /// Returns a StdResult<CosmosMsg> used to execute RegisterReceive
    ///
    /// # Arguments
    ///
    /// * `code_hash` - String holding code hash contract to be called when sent tokens
    pub fn register_receive_msg(&self, code_hash: String) -> StdResult<CosmosMsg> {
        register_receive_msg(
            code_hash,
            None,
            BLOCK_SIZE,
            self.code_hash.clone(),
            self.address.clone(),
        )
    }

    /// Returns a StdResult<TokenInfo> from performing TokenInfo query
    ///
    /// # Arguments
    ///
    /// * `querier` - a reference to the Querier dependency of the querying contract
    pub fn token_info_query<Q: Querier>(&self, querier: &Q) -> StdResult<TokenInfo> {
        token_info_query(
            querier,
            BLOCK_SIZE,
            self.code_hash.clone(),
            self.address.clone(),
        )
    }
}
