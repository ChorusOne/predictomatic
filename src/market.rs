// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

//! Type-safe wrappers for interactions with markets.

use crate::config::{MarketConfig, MarketKind};
use crate::database::{self as db, Transaction};
use crate::finance::{AssetId, MarketId};

type Result<T> = db::Result<T>;
#[derive(Copy, Clone, Debug)]
pub struct OutcomeId(i64, MarketId);

/// Convert an outcome to its correcponding asset.
///
/// In the database, the id for an asset is the id of the outcome, but in
/// addition to outcome assets, we have the native asset "points" with id 0.
impl From<OutcomeId> for AssetId {
    fn from(outcome_id: OutcomeId) -> AssetId {
        AssetId(outcome_id.0)
    }
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
