use cosmwasm_std::{to_binary, Api, Env, Extern, HandleResponse, InitResponse, Querier, Storage, HumanAddr, QueryResult, HandleResult, InitResult};

use crate::msg::{HandleMsg, InitMsg, QueryMsg, QueryResponse, History, CONFIG_KEY};
use crate::state::{State, save, load};
use std::collections::HashMap;

////////////////////////////////////// Init ///////////////////////////////////////
/// Initializes the oracle state
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `msg` - InitMsg passed in with the instantiation message
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    _msg: InitMsg,
) -> InitResult {
    let state = State {
        histories: HashMap::new(),
        owner: env.message.sender,
    };

    save(&mut deps.storage, CONFIG_KEY, &state)?;

    Ok(InitResponse::default())
}

///////////////////////////////////// Handle //////////////////////////////////////
/// Handle incoming messages from nodes
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
    match msg {
        HandleMsg::AddHistory {user, history} => try_add_history(deps, env, user, history),
    }
}

/// Add credit history for user
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `user` - current user to add history
/// * `history` - user history
pub fn try_add_history<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    user: HumanAddr,
    history: History,
) -> HandleResult {
    let mut state: State = load(&deps.storage, CONFIG_KEY)?;
    state.histories.insert(user, history);
    save(&mut deps.storage, CONFIG_KEY, &state)?;

    print!("History for user added successfully");
    Ok(HandleResponse::default())
}

/////////////////////////////////////// Query /////////////////////////////////////
/// Returns QueryResult
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
/// * `msg` - QueryMsg passed in with the query call
pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> QueryResult {
    match msg {
        QueryMsg::GetHistory {user} => query_get_history(deps, user),
    }
}

fn query_get_history<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    user: HumanAddr
) -> QueryResult {
    let state: State = load(&deps.storage, CONFIG_KEY)?;
    let hists = state.histories;
    if hists.contains_key(&user) {
        let history = &hists[&user];
        to_binary(&QueryResponse { history: Some(History{ debts: history.debts, credits: history.credits.clone()}), message: String::from("History for user found")})
    } else {
        to_binary(&QueryResponse { history: None, message: String::from("No history for user found")})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MockStorage, MockApi, MockQuerier};
    use cosmwasm_std::{from_binary, Uint128};
    use crate::msg::Credit;

    fn init_helper() -> (
            InitResult,
            Extern<MockStorage, MockApi, MockQuerier>,
    ) {
        let mut deps = mock_dependencies(20, &[]);
        let env = mock_env("oracle", &[]);

        let init_msg = InitMsg {user: None, history: None};
        (init(&mut deps, env, init_msg), deps)
    }

    #[test]
    fn add_history() {
        let (init_result, mut deps) = init_helper();
        assert!(
            init_result.is_ok(),
            "Init failed: {}",
            init_result.err().unwrap()
        );

        let credit1 = Credit {
            sum : Uint128(1000000),
            interest_rate : Uint128(6),
            time : Uint128(12),
            is_closed : true
        };
        let credit2 = Credit {
            sum : Uint128(100000),
            interest_rate : Uint128(5),
            time : Uint128(6),
            is_closed : false
        };
        let credit3 = Credit {
            sum : Uint128(500000),
            interest_rate : Uint128(10),
            time : Uint128(9),
            is_closed : false
        };
        let credit4 = Credit {
            sum : Uint128(200000),
            interest_rate : Uint128(7),
            time : Uint128(6),
            is_closed : true
        };
        let credit5 = Credit {
            sum : Uint128(150000),
            interest_rate : Uint128(10),
            time : Uint128(12),
            is_closed : true
        };

        let history = History{debts: Uint128(100000), credits: vec![credit1, credit2, credit3, credit4, credit5]};
        let user = HumanAddr("user".to_string());

        let handle_msg = HandleMsg::AddHistory { user, history};
        let _handle_result = handle(&mut deps, mock_env("oracle", &[]), handle_msg);

        let user = HumanAddr("user".to_string());
        let query_result = query(&deps, QueryMsg::GetHistory {user}).unwrap();
        let value: QueryResponse = from_binary(&query_result).unwrap();
        assert_ne!(None, value.history);
    }
}
