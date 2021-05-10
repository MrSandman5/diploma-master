use cosmwasm_std::{
    log, to_binary, Api, CanonicalAddr, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    InitResponse, InitResult, Querier, QueryResult, StdError, Storage, Uint128,
};

use std::collections::HashSet;

use serde_json_wasm as serde_json;

use secret_toolkit::utils::{pad_handle_result, pad_query_result};

use crate::msg::{HandleAnswer, HandleMsg, InitMsg, QueryAnswer, QueryMsg, ResponseStatus, ResponseStatus::{Failure, Success}, Token, CONFIG_KEY, BLOCK_SIZE, Credit, Proposition};
use crate::state::{load, may_load, remove, save, Bid, State};
use chrono::NaiveDateTime;

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
        let sum : u128 = (credit.sum * credit.interest_rate as u128) / (credit.time * 30 * 24) as u128;
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
        //HandleMsg::RetractBid { .. } => try_retract(deps, env.message.sender),
        HandleMsg::Finalize { only_if_bids, .. } => try_finalize(deps, env, only_if_bids, false),
        HandleMsg::ReturnAll { .. } => try_finalize(deps, env, false, true),
        HandleMsg::ReceiveConsign { from, amount, .. } => try_receive_consign(deps, env, from, amount),
        HandleMsg::ReceiveBid { from, amount, .. } => try_receive_bid(deps, env, from, amount),
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

/// Returns HandleResult
///
/// process the Receive message sent after sell token contract sent tokens to
/// auction escrow
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `from` - address of owner of tokens sent to escrow
/// * `amount` - Uint128 amount sent to escrow
fn try_receive_consign<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    from: HumanAddr,
    amount: Uint128,
) -> HandleResult {
    let mut state: State = load(&deps.storage, CONFIG_KEY)?;

    if env.message.sender == state.sell_contract.address {
        try_consign(deps, from, amount, &mut state)
    } else {
        let message = format!(
            "Address: {} is not a token in this auction",
            env.message.sender
        );
        let resp = serde_json::to_string(&HandleAnswer::Status {
            status: Failure,
            message,
        }).unwrap();

        Ok(HandleResponse {
            messages: vec![],
            log: vec![log("response", resp)],
            data: None,
        })
    }
}

/// Returns HandleResult
///
/// process the Receive message sent after bid
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `from` - address of owner of tokens sent to escrow
/// * `amount` - Proposition for bid
fn try_receive_bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    from: HumanAddr,
    amount: Proposition,
) -> HandleResult {
    let mut state: State = load(&deps.storage, CONFIG_KEY)?;

    if env.message.sender == state.bid_contract.address {
        try_bid(deps, env, from, amount, &mut state)
    } else {
        let message = format!(
            "Address: {} is not a token in this auction",
            env.message.sender
        );
        let resp = serde_json::to_string(&HandleAnswer::Status {
            status: Failure,
            message,
        }).unwrap();

        Ok(HandleResponse {
            messages: vec![],
            log: vec![log("response", resp)],
            data: None,
        })
    }
}

/// Returns HandleResult
///
/// process the attempt to consign sale tokens to auction escrow
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `owner` - address of owner of tokens sent to escrow
/// * `amount` - Uint128 amount sent to escrow
fn try_consign<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    owner: HumanAddr,
    amount: Uint128,
    state: &mut State,
) -> HandleResult {
    // if not the auction owner, send the tokens back
    if owner != state.seller {
        let message = String::from(
            "Only auction creator can consign tokens for sale. Your tokens have been returned",
        );

        let resp = serde_json::to_string(&HandleAnswer::Consign {
            status: Failure,
            message,
            amount_consigned: None,
            amount_needed: None,
            amount_returned: Some(amount),
        }).unwrap();

        return Ok(HandleResponse {
            messages: vec![state.sell_contract.transfer_msg(owner, amount)?],
            log: vec![log("response", resp)],
            data: None,
        });
    }
    // if auction is over, send the tokens back
    if state.is_completed {
        let message = String::from("Auction has ended. Your tokens have been returned");

        let resp = serde_json::to_string(&HandleAnswer::Consign {
            status: Failure,
            message,
            amount_consigned: None,
            amount_needed: None,
            amount_returned: Some(amount),
        }).unwrap();

        return Ok(HandleResponse {
            messages: vec![state.sell_contract.transfer_msg(owner, amount)?],
            log: vec![log("response", resp)],
            data: None,
        });
    }
    // if tokens to be sold have already been consigned, return these tokens
    if state.tokens_consigned {
        let message = String::from(
            "Tokens to be sold have already been consigned. Your tokens have been returned",
        );

        let resp = serde_json::to_string(&HandleAnswer::Consign {
            status: Failure,
            message,
            amount_consigned: Some(Uint128(state.currently_consigned)),
            amount_needed: None,
            amount_returned: Some(amount),
        }).unwrap();

        return Ok(HandleResponse {
            messages: vec![state.sell_contract.transfer_msg(owner, amount)?],
            log: vec![log("response", resp)],
            data: None,
        });
    }

    let consign_total = state.currently_consigned + amount.u128();
    let mut log_msg = String::new();
    let mut cos_msg = Vec::new();
    let status: ResponseStatus;
    let mut excess: Option<Uint128> = None;
    let mut needed: Option<Uint128> = None;
    // if consignment amount < auction sell amount, ask for remaining balance
    if consign_total < state.sell_amount {
        state.currently_consigned = consign_total;
        needed = Some(Uint128(state.sell_amount - consign_total));
        status = Failure;
        log_msg.push_str(
            "You have not consigned the full amount to be sold.  You need to consign additional \
             tokens",
        );
        // all tokens to be sold have been consigned
    } else {
        state.tokens_consigned = true;
        state.currently_consigned = state.sell_amount;
        status = Success;
        log_msg.push_str("Tokens to be sold have been consigned to the auction");
        // if consigned more than needed, return excess tokens
        if consign_total > state.sell_amount {
            excess = Some(Uint128(consign_total - state.sell_amount));
            cos_msg.push(state.sell_contract.transfer_msg(owner, excess.unwrap())?);
            log_msg.push_str(".  Excess tokens have been returned");
        }
    }

    save(&mut deps.storage, CONFIG_KEY, &state)?;

    let resp = serde_json::to_string(&HandleAnswer::Consign {
        status,
        message: log_msg,
        amount_consigned: Some(Uint128(state.currently_consigned)),
        amount_needed: needed,
        amount_returned: excess,
    }).unwrap();

    Ok(HandleResponse {
        messages: cos_msg,
        log: vec![log("response", resp)],
        data: None,
    })
}

/// Returns HandleResult
///
/// process the bid attempt
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `bidder` - address of owner of tokens sent to escrow
/// * `amount` - Uint128 amount sent to escrow
/// * `state` - mutable reference to auction state
fn try_bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    bidder: HumanAddr,
    amount: Proposition,
    state: &mut State,
) -> HandleResult {
    let tokens : Uint128 = Uint128((amount.sum * amount.interest_rate as u128) / (amount.time * 30 * 24) as u128);
    // if auction is over, send the tokens back
    if state.is_completed {
        let message = String::from("Auction has ended. Bid tokens have been returned");

        let resp = serde_json::to_string(&HandleAnswer::Bid {
            status: Failure,
            message,
            previous_bid: None,
            amount_bid: None,
            amount_returned: Some(tokens),
        }).unwrap();

        return Ok(HandleResponse {
            messages: vec![state.bid_contract.transfer_msg(bidder, tokens)?],
            log: vec![log("response", resp)],
            data: None,
        });
    }
    // don't accept a 0 bid
    if tokens == Uint128(0) {
        let message = String::from("Bid must be greater than 0");

        let resp = serde_json::to_string(&HandleAnswer::Bid {
            status: Failure,
            message,
            previous_bid: None,
            amount_bid: None,
            amount_returned: None,
        }).unwrap();

        return Ok(HandleResponse {
            messages: vec![],
            log: vec![log("response", resp)],
            data: None,
        });
    }
    // if bid is less than the minimum accepted bid, send the tokens back
    if tokens.u128() >= state.sell_amount {
        let message =
            String::from("Bid was bigger than stated. Bid tokens have been returned");

        let resp = serde_json::to_string(&HandleAnswer::Bid {
            status: Failure,
            message,
            previous_bid: None,
            amount_bid: None,
            amount_returned: Some(tokens),
        }).unwrap();

        return Ok(HandleResponse {
            messages: vec![state.bid_contract.transfer_msg(bidder, tokens)?],
            log: vec![log("response", resp)],
            data: None,
        });
    }
    let mut return_amount: Option<Uint128> = None;
    let bidder_raw = &deps.api.canonical_address(&bidder)?;

    // if there is an active bid from this address
    if state.bidders.contains(&bidder_raw.as_slice().to_vec()) {
        let bid: Option<Bid> = may_load(&deps.storage, bidder_raw.as_slice())?;
        if let Some(old_bid) = bid {
            // if new bid is <= the old bid, keep old bid and return this one
            if tokens.u128() <= old_bid.amount {
                let message = String::from(
                    "New bid less than or equal to previous bid. Newly bid tokens have been \
                     returned",
                );

                let resp = serde_json::to_string(&HandleAnswer::Bid {
                    status: Failure,
                    message,
                    previous_bid: Some(Uint128(old_bid.amount)),
                    amount_bid: None,
                    amount_returned: Some(tokens),
                }).unwrap();

                return Ok(HandleResponse {
                    messages: vec![state.bid_contract.transfer_msg(bidder, tokens)?],
                    log: vec![log("response", resp)],
                    data: None,
                });
                // new bid is larger, save the new bid, and return the old one, so mark for return
            } else {
                return_amount = Some(Uint128(old_bid.amount));
            }
        }
        // address did not have an active bid
    } else {
        // insert in list of bidders and save
        state.bidders.insert(bidder_raw.as_slice().to_vec());
        save(&mut deps.storage, CONFIG_KEY, &state)?;
    }
    let new_bid = Bid {
        amount: tokens.u128(),
        timestamp: env.block.time,
    };
    save(&mut deps.storage, bidder_raw.as_slice(), &new_bid)?;

    let mut message = String::from("Bid accepted");
    let mut cos_msg = Vec::new();

    // if need to return the old bid
    if let Some(returned) = return_amount {
        cos_msg.push(state.bid_contract.transfer_msg(bidder, returned)?);
        message.push_str(". Previously bid tokens have been returned");
    }
    let resp = serde_json::to_string(&HandleAnswer::Bid {
        status: Success,
        message,
        previous_bid: None,
        amount_bid: Some(tokens),
        amount_returned: return_amount,
    }).unwrap();

    Ok(HandleResponse {
        messages: cos_msg,
        log: vec![log("response", resp)],
        data: None,
    })
}

/// Returns HandleResult
///
/// closes the auction and sends all the tokens in escrow to where they belong
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `only_if_bids` - true if auction should stay open if there are no bids
/// * `return_all` - true if being called from the return_all fallback plan
fn try_finalize<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    only_if_bids: bool,
    return_all: bool,
) -> HandleResult {
    let mut state: State = load(&deps.storage, CONFIG_KEY)?;

    // can only do a return_all if the auction is closed
    if return_all && !state.is_completed {
        return Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::CloseAuction {
                status: Failure,
                message: String::from(
                    "return_all can only be executed after the auction has ended",
                ),
                winning_bid: None,
                amount_returned: None,
            })?),
        });
    }
    // if not the auction owner, can't finalize, but you can return_all
    if !return_all && env.message.sender != state.seller {
        return Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::CloseAuction {
                status: Failure,
                message: String::from("Only auction creator can finalize the sale"),
                winning_bid: None,
                amount_returned: None,
            })?),
        });
    }
    // if there are no active bids, and owner only wants to close if bids
    if !state.is_completed && only_if_bids && state.bidders.is_empty() {
        return Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::CloseAuction {
                status: Failure,
                message: String::from("Did not close because there are no active bids"),
                winning_bid: None,
                amount_returned: None,
            })?),
        });
    }
    let mut cos_msg = Vec::new();
    let mut update_state = false;
    let mut winning_amount: Option<Uint128> = None;
    let mut amount_returned: Option<Uint128> = None;

    let no_bids = state.bidders.is_empty();
    // if there were bids
    if !no_bids {
        // load all the bids
        struct OwnedBid {
            pub bidder: CanonicalAddr,
            pub bid: Bid,
        }
        let mut bid_list: Vec<OwnedBid> = Vec::new();
        for bidder in &state.bidders {
            let bid: Option<Bid> = may_load(&deps.storage, bidder.as_slice())?;
            if let Some(found_bid) = bid {
                bid_list.push(OwnedBid {
                    bidder: CanonicalAddr::from(bidder.as_slice()),
                    bid: found_bid,
                });
            }
        }
        // closing an auction that has been fully consigned
        if state.tokens_consigned && !state.is_completed {
            bid_list.sort_by(|a, b| {
                a.bid
                    .amount
                    .cmp(&b.bid.amount)
                    .then(b.bid.timestamp.cmp(&a.bid.timestamp))
            });
            // if there was a winner, swap the tokens
            if let Some(winning_bid) = bid_list.pop() {
                cos_msg.push(
                    state
                        .bid_contract
                        .transfer_msg(state.seller.clone(), Uint128(winning_bid.bid.amount))?,
                );
                cos_msg.push(state.sell_contract.transfer_msg(
                    deps.api.human_address(&winning_bid.bidder)?,
                    Uint128(state.sell_amount),
                )?);
                state.currently_consigned = 0;
                update_state = true;
                winning_amount = Some(Uint128(winning_bid.bid.amount));
                state.winning_bid = winning_bid.bid.amount;
                remove(&mut deps.storage, &winning_bid.bidder.as_slice());
                state
                    .bidders
                    .remove(&winning_bid.bidder.as_slice().to_vec());
            }
        }
        // loops through all remaining bids to return them to the bidders
        for losing_bid in &bid_list {
            cos_msg.push(state.bid_contract.transfer_msg(
                deps.api.human_address(&losing_bid.bidder)?,
                Uint128(losing_bid.bid.amount),
            )?);
            remove(&mut deps.storage, &losing_bid.bidder.as_slice());
            update_state = true;
            state.bidders.remove(&losing_bid.bidder.as_slice().to_vec());
        }
    }
    // return any tokens that have been consigned to the auction owner (can happen if owner
    // finalized the auction before consigning the full sale amount or if there were no bids)
    if state.currently_consigned > 0 {
        cos_msg.push(
            state
                .sell_contract
                .transfer_msg(state.seller.clone(), Uint128(state.currently_consigned))?,
        );
        if !return_all {
            amount_returned = Some(Uint128(state.currently_consigned));
        }
        state.currently_consigned = 0;
        update_state = true;
    }
    // mark that auction had ended
    if !state.is_completed {
        state.is_completed = true;
        update_state = true;
    }
    if update_state {
        save(&mut deps.storage, CONFIG_KEY, &state)?;
    }

    let log_msg = if winning_amount.is_some() {
        "Sale finalized.  You have been sent the winning bid tokens".to_string()
    } else if amount_returned.is_some() {
        let cause = if !state.tokens_consigned {
            " because you did not consign the full sale amount"
        } else if no_bids {
            " because there were no active bids"
        } else {
            ""
        };
        format!(
            "Auction closed.  You have been returned the consigned tokens{}",
            cause
        )
    } else if return_all {
        "Outstanding funds have been returned".to_string()
    } else {
        "Auction has been closed".to_string()
    };
    Ok(HandleResponse {
        messages: cos_msg,
        log: vec![],
        data: Some(to_binary(&HandleAnswer::CloseAuction {
            status: Success,
            message: log_msg,
            winning_bid: winning_amount,
            amount_returned,
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
        description: state.description,
        auction_address: state.auction_addr,
        status,
        winning_bid,
    })
}



