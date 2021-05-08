use cosmwasm_std::{debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdError, StdResult, Storage, InitResult, Uint128, HandleResult, HumanAddr, QueryResult};

use crate::msg::{CountResponse, HandleMsg, InitMsg, QueryMsg, ResponseStatus, HandleAnswer, QueryAnswer, Token, Credit};
use crate::state::{config, config_read, State, save, load, Bid, may_load};

use crate::msg::{CONFIG_KEY, BLOCK_SIZE};
use std::collections::HashSet;
use crate::msg::ResponseStatus::{Success, Failure};

////////////////////////////////////// Init ///////////////////////////////////////
/// Returns InitResult
///
/// Initializes the auction state and registers Receive function with sell and bid
/// token contracts
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `msg` - InitMsg passed in with the instantiation message
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> InitResult {
    let mut current_amount : u128 = 0;
    if msg.sell_amount.len() == 0 {
        current_amount = 5;
    } else {
        current_amount = convert_to_tokens(msg.sell_amount);
    }
    if msg.sell_contract.address == msg.bid_contract.address {
        return Err(StdError::generic_err(
            "Sell contract and bid contract must be different",
        ));
    }
    let state = State {
        auction_addr: env.contract.address,
        seller: env.message.sender,
        sell_contract: msg.sell_contract,
        bid_contract: msg.bid_contract,
        sell_amount: current_amount,
        currently_consigned: 0,
        bidders: HashSet::new(),
        is_completed: false,
        tokens_consigned: false,
        description: msg.description,
        winning_bid: 0,
    };

    save(&mut deps.storage, CONFIG_KEY, &state)?;

    // register receive with the bid/sell token contracts
    Ok(InitResponse {
        messages: vec![
            state
                .sell_contract
                .register_receive_msg(env.contract_code_hash.clone())?,
            state
                .bid_contract
                .register_receive_msg(env.contract_code_hash)?,
        ],
        log: vec![],
    })
}

fn convert_to_tokens(history : Vec<Credit>) -> u128 {
    let mut tokens : u128 = 0;
    for credit in history {
        let sum : u128 = (credit.sum * credit.interest_rate) / (credit.time * 30 * 24);
        if credit.is_closed {
            tokens += sum;
        } else { tokens -= sum; }
    }
    if tokens < 0 {
        5
    } else { tokens }
}

///////////////////////////////////// Handle //////////////////////////////////////
/// Returns HandleResult
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `msg` - HandleMsg passed in with the execute message
pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    let response = match msg {
        HandleMsg::RetractBid { .. } => try_retract(deps, env.message.sender),
        HandleMsg::Finalize { only_if_bids, .. } => try_finalize(deps, env, only_if_bids, false),
        HandleMsg::ReturnAll { .. } => try_finalize(deps, env, false, true),
        HandleMsg::Receive { from, amount, .. } => try_receive(deps, env, from, amount),
        HandleMsg::ViewBid { .. } => try_view_bid(deps, &env.message.sender)
    };
    pad_handle_result(response, BLOCK_SIZE)
}

/// Returns HandleResult
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `bidder` - reference to address wanting to view its bid
fn try_view_bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    bidder: &HumanAddr,
) -> HandleResult {
    let state: State = load(&deps.storage, CONFIG_KEY)?;

    let bidder_raw = &deps.api.canonical_address(bidder)?;
    let mut amount_bid: Option<Uint128> = None;
    let mut message = String::new();
    let status: ResponseStatus;

    if state.bidders.contains(&bidder_raw.as_slice().to_vec()) {
        let bid: Option<Bid> = may_load(&deps.storage, bidder_raw.as_slice())?;
        if let Some(found_bid) = bid {
            status = Success;
            amount_bid = Some(Uint128(found_bid.amount));
            message.push_str(&format!(
                "Bid placed {} UTC",
                NaiveDateTime::from_timestamp(found_bid.timestamp as i64, 0)
                    .format("%Y-%m-%d %H:%M:%S")
            ));
        } else {
            status = Failure;
            message.push_str(&format!("No active bid for address: {}", bidder));
        }
        // no active bid found
    } else {
        status = Failure;
        message.push_str(&format!("No active bid for address: {}", bidder));
    }
    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Bid {
            status,
            message,
            previous_bid: None,
            amount_bid,
            amount_returned: None,
        })?),
    })
}

/////////////////////////////////////// Query /////////////////////////////////////
/// Returns QueryResult
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
/// * `msg` - QueryMsg passed in with the query call
pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    let response = match msg {
        QueryMsg::AuctionInfo { .. } => try_query_info(deps),
    };
    pad_query_result(response, BLOCK_SIZE)
}

/// Returns QueryResult
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
fn try_query_info<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> QueryResult {
    let state: State = load(&deps.storage, CONFIG_KEY)?;

    // get sell token info
    let sell_token_info = state.sell_contract.token_info_query(&deps.querier)?;
    // get bid token info
    let bid_token_info = state.bid_contract.token_info_query(&deps.querier)?;

    // build status string
    let status = if state.is_completed {
        let locked = if !state.bidders.is_empty() || state.currently_consigned > 0 {
            ", but found outstanding balances.  Please run either retract_bid to \
                retrieve your non-winning bid, or return_all to return all outstanding bids/\
                consignment."
        } else {
            ""
        };
        format!("Closed{}", locked)
    } else {
        let consign = if !state.tokens_consigned { " NOT" } else { "" };
        format!(
            "Accepting bids: Token(s) to be sold have{} been consigned to the auction",
            consign
        )
    };

    let winning_bid = if state.winning_bid == 0 {
        None
    } else {
        Some(Uint128(state.winning_bid))
    };

    to_binary(&QueryAnswer::AuctionInfo {
        sell_token: Token {
            contract_address: state.sell_contract.address,
            token_info: sell_token_info,
        },
        bid_token: Token {
            contract_address: state.bid_contract.address,
            token_info: bid_token_info,
        },
        sell_amount: Uint128(state.sell_amount),
        minimum_bid: Uint128(state.minimum_bid),
        description: state.description,
        auction_address: state.auction_addr,
        status,
        winning_bid,
    })
}



