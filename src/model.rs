// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

//! Type-safe wrappers for interactions with accounts and ledgers.

use std::fmt;

use crate::config::{AppConfig, MarketConfig, MarketKind};
use crate::database::{self as db, Transaction};

type Result<T> = db::Result<T>;

#[derive(Copy, Clone, Debug)]
pub struct MarketId(i64);

impl MarketId {
    /// Market 0 is the global "no market" for points accounts outside of a market.
    ///
    /// All accounts that hold assets are related to a market, except for an
    /// owner global points balance. We represent that as a market anyway, to
    /// make things more uniform.
    pub const NONE: MarketId = MarketId(0);
}

#[derive(Copy, Clone, Debug)]
pub struct OutcomeId(i64, MarketId);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AssetId(i64);

impl AssetId {
    /// The asset id for the native asset ("points").
    pub const POINTS: AssetId = AssetId(0);

    pub fn zero(&self) -> Amount {
        Amount(0, *self)
    }

    pub fn parse_amount(&self, s: &str) -> Option<Amount> {
        use std::str::FromStr;

        let mut parts = s.split('.');
        let integral = i64::from_str(parts.next()?).ok()?;

        let fractional = match parts.next() {
            // No fractional part.
            None => 0,
            // The input is more precise than we can handle.
            Some(frac_str) if frac_str.len() > 6 => return None,
            // We have decimals, scale to micros.
            Some(frac_str) => {
                let frac_int = i64::from_str(frac_str).ok()?;
                frac_int * 10_i64.pow(6 - frac_str.len() as u32)
            }
        };

        if parts.next().is_some() {
            // We expect at most two one decimal point, so two parts.
            // If there is a third part, the input is invalid.
            return None;
        }

        Some(Amount(
            integral * 1_000_000 + fractional * integral.signum(),
            *self,
        ))
    }
}

/// Convert an outcome to its correcponding asset.
///
/// In the database, the id for an asset is the id of the outcome, but in
/// addition to outcome assets, we have the native asset "points" with id 0.
impl From<OutcomeId> for AssetId {
    fn from(outcome_id: OutcomeId) -> AssetId {
        AssetId(outcome_id.0)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AccountId(i64, AssetId);

impl AccountId {
    /// The account id for the system's points account.
    pub const SYSTEM_POINTS: AccountId = AccountId(0, AssetId::POINTS);
}

/// An amount of a given asset.
///
/// The integer represents a micro-increment of the asset, i.e. 10^-6.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Amount(i64, AssetId);

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Unless a different format was selected, print in full precision.
        let precision = match f.precision() {
            Some(n) => n,
            None => 6,
        };

        debug_assert!(
            precision <= 6,
            "Amounts have at most 6 decimal digits of precision."
        );

        // Amounts are in micros, so we have 6 decimals by default.
        let integral = self.0 / 1_000_000;
        let fractional = (self.0 % 1_000_000).abs();

        // Round to the requested number of decimal places.
        let pow10_trunc = 10_i64.pow(6 - precision as u32);
        let fractional_trunc = (fractional + pow10_trunc / 2) / pow10_trunc;

        write!(f, "{integral}.{fractional_trunc:>0p$}", p = precision)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EventId(i64);

/// Create a transfer from one account to another.
pub fn create_transfer(tx: &mut Transaction, event: EventId, from_account: AccountId, to_account: AccountId, amount: Amount) -> Result<()> {
    assert_eq!(from_account.1, to_account.1, "Asset must be the same for both accounts.");
    assert_eq!(from_account.1, amount.1, "Asset must be the same for accounts and amount.");
    assert!(amount.0 > 0, "Transfer must be positive.");
    db::create_transfer(tx, event.0, from_account.0, to_account.0, amount.0)
}

/// Ensure that an account exists for the given (market_id, owner_id, asset_id), return its id.
pub fn ensure_account(
    tx: &mut Transaction,
    market_id: MarketId,
    asset_id: AssetId,
    owner: &str,
) -> Result<AccountId> {
    let id = match db::get_account_id(tx, market_id.0, asset_id.0, owner)? {
        Some(id) => id,
        None => db::create_account(tx, market_id.0, asset_id.0, owner)?,
    };
    Ok(AccountId(id, asset_id))
}

/// Ensure that a user has a global points account, fund it with the opening balance if needed, return id.
pub fn ensure_points_account(
    tx: &mut Transaction,
    config: &AppConfig,
    owner: &str,
) -> Result<AccountId> {
    let market_id = MarketId::NONE;
    let asset_id = AssetId::POINTS;
    let id = match db::get_account_id(tx, market_id.0, asset_id.0, owner)? {
        Some(id) => id,
        None => {
            let to_account = AccountId(db::create_account(tx, market_id.0, asset_id.0, owner)?, asset_id);
            let from_account = AccountId::SYSTEM_POINTS;
            let event = EventId(db::create_event(tx, "SYSTEM", "Sign-on bonus")?);
            let amount = Amount(config.opening_balance_micropoints, asset_id);
            create_transfer(tx, event, from_account, to_account, amount)?;
            to_account.0
        }
    };
    Ok(AccountId(id, asset_id))
}

pub fn get_account_balance(tx: &mut Transaction, account_id: AccountId) -> Result<Amount> {
    let amount = db::get_account_balance(tx, account_id.0)?;
    Ok(Amount(amount, account_id.1))
}

pub struct Outcome {
    pub id: OutcomeId,
    pub value: String,
}

pub struct Market {
    pub id: MarketId,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub kind: MarketKind,
    pub outcomes: Vec<Outcome>,
}

pub fn get_market_by_slug(tx: &mut Transaction, slug: &str) -> Result<Option<Market>> {
    let market = match db::get_market_by_slug(tx, slug)? {
        Some(market) => market,
        None => return Ok(None),
    };

    let market_id = MarketId(market.id);
    let kind: MarketKind =
        serde_plain::from_str(&market.kind).expect("Invalid market kind in database.");

    let mut outcomes = Vec::new();
    for res_outcome in db::get_outcomes(tx, market.id)? {
        let outcome = res_outcome?;
        outcomes.push(Outcome {
            id: OutcomeId(outcome.id, market_id),
            value: outcome.value,
        });
    }

    Ok(Some(Market {
        id: market_id,
        slug: market.slug,
        title: market.title,
        description: market.description,
        kind,
        outcomes,
    }))
}

pub fn create_market(tx: &mut Transaction, market: &MarketConfig) -> Result<MarketId> {
    let kind = market.kind.to_string();
    let market_id = db::create_market(tx, &market.slug, &kind, &market.title, &market.description)?;

    for outcome in &market.outcomes {
        db::create_outcome(tx, market_id, outcome)?;
    }

    Ok(MarketId(market_id))
}

/// Create the configured markets if they don't yet exist.
pub fn ensure_markets(tx: &mut Transaction, markets: &[MarketConfig]) -> Result<()> {
    for market in markets {
        if get_market_by_slug(tx, &market.slug)?.is_some() {
            continue;
        }
        create_market(tx, market)?;
    }

    Ok(())
}

pub fn market_deposit(
    tx: &mut Transaction,
    market: &Market,
    amount: Amount,
    owner: &str,
) -> Result<()> {
    let event = EventId(db::create_event(tx, owner, "Deposit")?);

    // Move the points from the user's global account, into the user's point
    // account for this market.
    let acc_global_points = ensure_account(tx, MarketId::NONE, AssetId::POINTS, owner)?;
    let acc_market_points = ensure_account(tx, market.id, AssetId::POINTS, owner)?;
    create_transfer(tx, event, acc_global_points, acc_market_points, amount)?;

    // TODO: Mint the outcome shares.

    Ok(())
}

#[cfg(test)]
mod test {
    use super::{Amount, AssetId};

    #[test]
    fn amount_display_works() {
        let x = Amount(123_456_789, AssetId::POINTS);
        assert_eq!(format!("{x}"), "123.456789");
        assert_eq!(format!("{x:.3}"), "123.457");
        assert_eq!(format!("{x:.1}"), "123.5");

        let x = Amount(123_000_789, AssetId::POINTS);
        assert_eq!(format!("{x}"), "123.000789");
        assert_eq!(format!("{x:.3}"), "123.001");
        assert_eq!(format!("{x:.1}"), "123.0");

        let x = Amount(-123_456_789, AssetId::POINTS);
        assert_eq!(format!("{x}"), "-123.456789");
        assert_eq!(format!("{x:.3}"), "-123.457");
        assert_eq!(format!("{x:.1}"), "-123.5");

        let x = Amount(-123_000_789, AssetId::POINTS);
        assert_eq!(format!("{x}"), "-123.000789");
        assert_eq!(format!("{x:.3}"), "-123.001");
        assert_eq!(format!("{x:.1}"), "-123.0");
    }

    #[test]
    fn asset_id_parse_amount_works() {
        assert_eq!(
            AssetId::POINTS.parse_amount("1"),
            Some(Amount(1_000_000, AssetId::POINTS))
        );
        assert_eq!(
            AssetId::POINTS.parse_amount("1.0"),
            Some(Amount(1_000_000, AssetId::POINTS))
        );
        assert_eq!(
            AssetId::POINTS.parse_amount("1.2"),
            Some(Amount(1_200_000, AssetId::POINTS))
        );
        assert_eq!(
            AssetId::POINTS.parse_amount("1.02"),
            Some(Amount(1_020_000, AssetId::POINTS))
        );
        assert_eq!(
            AssetId::POINTS.parse_amount("1.000123"),
            Some(Amount(1_000_123, AssetId::POINTS))
        );
        assert_eq!(
            AssetId::POINTS.parse_amount("-1.2"),
            Some(Amount(-1_200_000, AssetId::POINTS))
        );
        assert_eq!(
            AssetId::POINTS.parse_amount("-1.02"),
            Some(Amount(-1_020_000, AssetId::POINTS))
        );
        assert_eq!(
            AssetId::POINTS.parse_amount("-1.000123"),
            Some(Amount(-1_000_123, AssetId::POINTS))
        );

        // Too many decimals, should refuse.
        assert_eq!(AssetId::POINTS.parse_amount("1.0001234"), None);
    }
}
