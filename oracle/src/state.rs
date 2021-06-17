use serde::{Deserialize, Serialize};

use cosmwasm_std::{Storage, HumanAddr, StdResult, ReadonlyStorage, StdError};
use crate::msg::History;
use std::collections::HashMap;
use secret_toolkit::serialization::{Bincode2, Serde};
use serde::de::DeserializeOwned;
use std::any::type_name;

/// state of the oracle
#[derive(Serialize, Deserialize, Clone)]
pub struct State {
    pub histories: HashMap<HumanAddr, History>,
    pub owner: HumanAddr,
}

pub fn save<T: Serialize, S: Storage>(storage: &mut S, key: &[u8], value: &T) -> StdResult<()> {
    storage.set(key, &Bincode2::serialize(value)?);
    Ok(())
}

pub fn load<T: DeserializeOwned, S: ReadonlyStorage>(storage: &S, key: &[u8]) -> StdResult<T> {
    Bincode2::deserialize(
        &storage
            .get(key)
            .ok_or_else(|| StdError::not_found(type_name::<T>()))?,
    )
}
