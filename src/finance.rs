// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

//! Type-safe wrappers for interactions with accounts and ledgers.

use crate::config::AppConfig;
use crate::database::{self as db, Transaction};

type Result<T> = db::Result<T>;

#[derive(Copy, Clone, Debug)]
pub struct AssetId(i64);

#[derive(Copy, Clone, Debug)]
pub struct AccountId(i64, AssetId);

impl AssetId {
    /// The asset id for the native asset ("points").
    const POINTS: AssetId = AssetId(0);
}

impl AccountId {
    /// The account id for the system's points account.
    const SYSTEM_POINTS: AccountId = AccountId(0, AssetId::POINTS);
}

/// An amount of a given asset.
///
/// The integer represents a micro-increment of the asset, i.e. 10^-6.
/// TODO: Remove the inner pub, expose formatters instead.
#[derive(Copy, Clone)]
pub struct Amount(pub i64, AssetId);

#[derive(Copy, Clone, Debug)]
pub struct EventId(i64);

/// Ensure that an account exists for the given (owner, asset_id) pair, return its id.
pub fn ensure_account(tx: &mut Transaction, owner: &str, asset_id: AssetId) -> Result<AccountId> {
    let id = match db::get_account_id(tx, owner, asset_id.0)? {
        Some(id) => id,
        None => db::create_account(tx, owner, asset_id.0)?,
    };
    Ok(AccountId(id, asset_id))
}

/// Ensure that a user has a points account, fund it with the opening balance if needed, return id.
pub fn ensure_points_account(
    tx: &mut Transaction,
    config: &AppConfig,
    owner: &str,
) -> Result<AccountId> {
    let asset_id = AssetId::POINTS;
    let id = match db::get_account_id(tx, owner, asset_id.0)? {
        Some(id) => id,
        None => {
            let to_account_id = db::create_account(tx, owner, asset_id.0)?;
            let from_account_id = AccountId::SYSTEM_POINTS.0;
            let event_id = db::create_event(tx, "SYSTEM", "Sign-on bonus")?;
            let amount = config.opening_balance_micropoints;
            db::create_transfer(tx, event_id, from_account_id, to_account_id, amount)?;
            to_account_id
        }
    };
    Ok(AccountId(id, asset_id))
}

pub fn get_account_balance(tx: &mut Transaction, account_id: AccountId) -> Result<Amount> {
    let amount = db::get_account_balance(tx, account_id.0)?;
    Ok(Amount(amount, account_id.1))
}
