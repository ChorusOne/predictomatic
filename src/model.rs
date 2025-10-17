// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

//! Type-safe wrappers for interactions with accounts and ledgers.

use std::collections::HashMap;
use std::fmt;

use crate::config::{AppConfig, MarketConfig, MarketKind};
use crate::database::{self as db, Transaction};

type Result<T> = db::Result<T>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
pub struct OutcomeId(pub i64, MarketId);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AssetId(pub i64);

impl AssetId {
    /// The asset id for the native asset ("points").
    pub const POINTS: AssetId = AssetId(0);

    pub fn zero(&self) -> Amount {
        Amount(0, *self)
    }

    /// Return `n` micro of the asset.
    pub fn micros(&self, n: i64) -> Amount {
        Amount(n, *self)
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

        let sign = if s.starts_with("-") { -1 } else { 1 };

        Some(Amount(integral * 1_000_000 + fractional * sign, *self))
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
#[derive(Copy, Clone, Debug)]
pub struct Amount(pub i64, pub AssetId);

impl Amount {
    /// Cast the amount to a different asset type.
    pub fn cast(&self, new_asset: AssetId) -> Amount {
        Amount(self.0, new_asset)
    }

    /// For an outcome asset, and an asset price (at points per share), return the value in points.
    pub fn value_at(self, price: f64) -> Amount {
        Amount((self.0 as f64 * price) as i64, AssetId::POINTS)
    }
}

impl std::ops::Add for Amount {
    type Output = Amount;
    fn add(self, rhs: Amount) -> Amount {
        assert_eq!(self.1, rhs.1, "Can only add amounts for the same asset.");
        // Always check for overflow, not just in debug mode.
        Amount(self.0.checked_add(rhs.0).unwrap(), self.1)
    }
}

impl std::ops::Sub for Amount {
    type Output = Amount;
    fn sub(self, rhs: Amount) -> Amount {
        assert_eq!(
            self.1, rhs.1,
            "Can only subtract amounts for the same asset."
        );
        // Always check for overflow, not just in debug mode.
        Amount(self.0.checked_sub(rhs.0).unwrap(), self.1)
    }
}

impl std::cmp::PartialOrd for Amount {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Technically with *partial* cmp we should return `None`, but even
        // attempting to compare is a bug so we should panic.
        assert_eq!(self.1, other.1, "Comparing amounts for different assets.");
        self.0.partial_cmp(&other.0)
    }
}

impl std::cmp::PartialEq for Amount {
    fn eq(&self, other: &Self) -> bool {
        assert_eq!(self.1, other.1, "Comparing amounts for different assets.");
        self.0 == other.0
    }
}

impl std::cmp::Eq for Amount {}

impl std::cmp::Ord for Amount {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        assert_eq!(self.1, other.1, "Comparing amounts for different assets.");
        self.0.cmp(&other.0)
    }
}

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

        // Round to the requested number of decimal places.
        let pow10_trunc = 10_i64.pow(6 - precision as u32);

        // Amounts are in micros, so we have 6 decimals by default.
        let amount = self.0 + (pow10_trunc / 2) * self.0.signum();
        let integral = amount / 1_000_000;
        let fractional = (amount % 1_000_000).abs();
        let fractional_trunc = fractional / pow10_trunc;

        write!(f, "{integral}.{fractional_trunc:>0p$}", p = precision)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EventId(i64);

/// Create a transfer from one account to another.
pub fn create_transfer(
    tx: &mut Transaction,
    event: EventId,
    from_account: AccountId,
    to_account: AccountId,
    amount: Amount,
) -> Result<()> {
    assert_eq!(
        from_account.1, to_account.1,
        "Asset must be the same for both accounts."
    );
    assert_eq!(
        from_account.1, amount.1,
        "Asset must be the same for accounts and amount."
    );
    assert!(amount.0 > 0, "Transfer must be positive.");
    db::create_transfer(tx, event.0, from_account.0, to_account.0, amount.0)
}

/// Constraints on account balance.
/// TODO: I don't actually use accounts that can have both,
/// I could simplify to credit/debit account and single column in db.
#[derive(Copy, Clone)]
enum AccountConstraint {
    /// Zero or positive.
    Positive,
    /// No constraints.
    Any,
    /// Zero or negative.
    Negative,
}

impl AccountConstraint {
    pub fn min_max_balance(&self) -> (Option<i64>, Option<i64>) {
        match self {
            AccountConstraint::Positive => (Some(0), None),
            AccountConstraint::Any => (None, None),
            AccountConstraint::Negative => (None, Some(0)),
        }
    }
}

/// Ensure that an account exists for the given (market_id, owner_id, asset_id), return its id.
pub fn ensure_account(
    tx: &mut Transaction,
    market_id: MarketId,
    asset_id: AssetId,
    owner: &str,
    constraint: AccountConstraint,
) -> Result<AccountId> {
    let (min, max) = constraint.min_max_balance();
    let id = match db::get_account_id(tx, market_id.0, asset_id.0, owner)? {
        Some(id) => id,
        None => db::create_account(tx, market_id.0, asset_id.0, owner, min, max)?,
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
            let (min, max) = AccountConstraint::Positive.min_max_balance();
            let to_account = AccountId(
                db::create_account(tx, market_id.0, asset_id.0, owner, min, max)?,
                asset_id,
            );
            let from_account = AccountId::SYSTEM_POINTS;
            let event = EventId(db::create_event(tx, "SYSTEM", "Sign-on bonus")?);
            let amount = Amount(config.opening_balance_micros, asset_id);
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

/// The balances of accounts related to a market for a given owner.
///
/// The owner can be a user, or it can be the system.
#[derive(Clone, Debug)]
pub struct Balance {
    /// For every outcome in the market, in the same order, the balance.
    pub outcomes: Vec<Amount>,

    /// Balance of the points account.
    pub points: Amount,
}

/// A probability distribution for the outcomes of a given market.
pub struct Distribution {
    /// The value of the LMSR invariant.
    ///
    /// The value of the invariant should be the same before and after the trade.
    invariant: f64,

    /// For every outcome, its log-probability.
    ///
    /// Invariant: the logits are normalized such that their exps sum to 1.
    logits: Vec<f64>,
}

impl Distribution {
    // TODO: Make the "B" parameter configurable per market.
    // Higher values make it harder to change the price.
    // For now we hard-code it to 10.0.
    // Need to keep in sync with js.
    const PARAM_B: f64 = 41.5;

    /// Turn AMM pool balances into a probability distribution.
    pub fn from_pool(balance: &Balance) -> Distribution {
        let mut logits: Vec<f64> = balance
            .outcomes
            .iter()
            // The points are stored as micros, correct for that to not blow up
            // the exps below.
            .map(|oc| -(oc.0 as f64) * 1e-6 / Self::PARAM_B)
            .collect();

        // In principle this is it, but when we convert to probability, we take
        // the exp of the logits and divide by their sum to normalize the
        // distribution. But dividing a log by a constant is just subtracting
        // a constant from the logit, so we can do that here already.
        let mut exps: Vec<_> = logits.iter().map(|lk| lk.exp()).collect();
        exps.sort_by(|x, y| x.partial_cmp(y).expect("Pool balances must not go to 0."));

        let invariant: f64 = exps.iter().sum();
        let ln_invariant = invariant.ln();
        debug_assert!(ln_invariant.is_finite());

        for lk in logits.iter_mut() {
            debug_assert!(lk.is_finite());
            *lk -= ln_invariant;
        }

        Distribution { invariant, logits }
    }

    /// Return the probability for every outcome.
    pub fn ps(&self) -> Vec<f64> {
        self.logits.iter().map(|k| k.exp()).collect()
    }
}

pub struct RealizedProfit {
    pub owner: String,
    pub amount_in: Amount,
    pub amount_out: Amount,
}

pub struct Market {
    pub id: MarketId,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub kind: MarketKind,
    pub outcomes: Vec<Outcome>,

    /// For every owner, their balance for this market.
    pub balances: HashMap<String, Balance>,

    /// For resolved markets, the realized profits per participant.
    pub profits: Vec<RealizedProfit>,
}

impl Market {
    /// Whether it is possible to trade in the market.
    // TODO: Replace with `outcome() -> Option<Outcome>` or something?
    pub fn is_open(&self) -> bool {
        self.profits.is_empty()
    }

    /// Return the amount of points deposited into this market.
    pub fn total_deposited(&self) -> Amount {
        let mut sum = AssetId::POINTS.zero();
        for b in self.balances.values() {
            sum = sum + b.points;
        }
        sum
    }

    /// Return the probability distribution over outcomes implied by the AMM pool balances.
    pub fn implied_distribution(&self) -> Distribution {
        Distribution::from_pool(&self.balances["SYSTEM"])
    }

    /// Trade against the pool.
    ///
    /// The input is the order, amount to sell + minimum output. The return
    /// value is the actual amount out, which will be at least `min_out`.
    ///
    /// Aside from computing the output amount, this validates that the amount
    /// assets belong to the market.
    pub fn trade(&self, amount_in: Amount, min_out: Amount) -> Option<Amount> {
        let pool_balance = &self.balances["SYSTEM"];
        let dist = Distribution::from_pool(pool_balance);

        // Get the current pool balances. Asset i is the one we sell to the pool,
        // asset j the one we get out.
        let q_i = match pool_balance.outcomes.iter().find(|oc| oc.1 == amount_in.1) {
            Some(b) => b,
            None => return None,
        };
        let q_j = match pool_balance.outcomes.iter().find(|oc| oc.1 == min_out.1) {
            Some(b) => b,
            None => return None,
        };

        let q_i_prime = *q_i + amount_in;
        let logit_i = (-q_i_prime.0 as f64) * 1e-6 / Distribution::PARAM_B;
        let q_j_prime = -Distribution::PARAM_B * (dist.invariant - logit_i.exp()).ln();

        // Convert the float back to micros again, round down the output
        // (in the AMMs advantage, the user's disadvantage) so that the user
        // cannot exploit rounding errors.
        let q_j_prime_int = (q_j_prime * 1e6).floor() as i64;
        let q_j_prime = Amount(q_j_prime_int, min_out.1);

        // TODO: Add a dedicated error for "the pool cannot afford this swap".
        if q_j_prime.0 < 0 {
            return None;
        }

        println!("i:{q_i} j:{q_j} -> i':{q_i_prime} j':{q_j_prime}");

        // The price may have changed since the user constructed the order, so
        // the order includes a slippage tolerance, we fail it if we don't get
        // the expected amount out.
        let delta = *q_j - q_j_prime;
        if delta >= min_out { Some(delta) } else { None }
    }
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

    let mut balances = HashMap::new();
    let mut default_balance = Balance {
        outcomes: outcomes
            .iter()
            .map(|oc| AssetId::from(oc.id).zero())
            .collect(),
        points: AssetId::POINTS.zero(),
    };

    for res_account in db::get_market_accounts(tx, market.id)? {
        let account = res_account?;
        let balance = balances
            .entry(account.owner)
            .or_insert_with(|| default_balance.clone());
        match account.asset_id {
            0 => balance.points.0 = account.balance,
            k => {
                // TODO: I should really use named fields rather than anonymous
                // tuples, this b.1.0 is ridiculous.
                let b = balance
                    .outcomes
                    .iter_mut()
                    .find(|b| b.1.0 == k)
                    .expect("We have an account, the oucome must exist.");
                b.0 = account.balance;
            }
        }
    }

    let mut profits = Vec::new();
    for res_profit in db::get_realized_profits(tx, market.id)? {
        let profit = res_profit?;
        profits.push(RealizedProfit {
            owner: profit.owner,
            amount_in: Amount(profit.amount_in, AssetId::POINTS),
            amount_out: Amount(profit.amount_out, AssetId::POINTS),
        });
    }

    Ok(Some(Market {
        id: market_id,
        slug: market.slug,
        title: market.title,
        description: market.description,
        kind,
        outcomes,
        balances,
        profits,
    }))
}

pub fn create_market(tx: &mut Transaction, market: &MarketConfig) -> Result<MarketId> {
    let kind = market.kind.to_string();
    let market_id = db::create_market(tx, &market.slug, &kind, &market.title, &market.description)?;

    for outcome in &market.outcomes {
        db::create_outcome(tx, market_id, outcome)?;
    }

    let fund_amount = AssetId::POINTS.micros(market.fund_micros);
    let market = get_market_by_slug(tx, &market.slug)?.expect("We created it, it exists.");
    create_deposit(tx, &market, fund_amount, "SYSTEM")?;

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

pub fn create_deposit(
    tx: &mut Transaction,
    market: &Market,
    amount: Amount,
    owner: &str,
) -> Result<()> {
    let event = EventId(db::create_event(tx, owner, "Deposit")?);

    // Move the points from the user's global account, into the user's point
    // account for this market.
    let pos = AccountConstraint::Positive;
    let neg = AccountConstraint::Negative;
    let acc_global_points = ensure_account(tx, MarketId::NONE, AssetId::POINTS, owner, pos)?;
    let acc_market_points = ensure_account(tx, market.id, AssetId::POINTS, owner, pos)?;
    create_transfer(tx, event, acc_global_points, acc_market_points, amount)?;

    // Mint the outcome shares equal to the deposited amount for every outcome.
    for outcome in &market.outcomes {
        let asset = outcome.id.into();
        let acc_mint = ensure_account(tx, MarketId::NONE, asset, "SYSTEM", neg)?;
        let acc_owner = ensure_account(tx, market.id, asset, owner, pos)?;
        create_transfer(tx, event, acc_mint, acc_owner, amount.cast(asset))?;
    }

    Ok(())
}

pub fn create_trade(
    tx: &mut Transaction,
    market: &Market,
    amount_in: Amount,
    amount_out: Amount,
    owner: &str,
) -> Result<()> {
    let event = EventId(db::create_event(tx, owner, "Trade")?);

    let pos = AccountConstraint::Positive;

    let acc_pool_0 = ensure_account(tx, market.id, amount_in.1, "SYSTEM", pos)?;
    let acc_user_0 = ensure_account(tx, market.id, amount_in.1, owner, pos)?;
    let acc_pool_1 = ensure_account(tx, market.id, amount_out.1, "SYSTEM", pos)?;
    let acc_user_1 = ensure_account(tx, market.id, amount_out.1, owner, pos)?;

    create_transfer(tx, event, acc_user_0, acc_pool_0, amount_in)?;
    create_transfer(tx, event, acc_pool_1, acc_user_1, amount_out)?;

    Ok(())
}

pub fn create_resolution(tx: &mut Transaction, market: &Market, outcome: OutcomeId) -> Result<()> {
    assert_eq!(outcome.1, market.id);

    // TODO: Have multiple admins and record which admin resolved?
    let event = EventId(db::create_event(tx, "SYSTEM", "Resolve")?);

    // As a first step, move all points that are escrowed in individual accounts
    // into the market's points account, so we can pay out from there.
    let pos = AccountConstraint::Positive;
    let pool_points = ensure_account(tx, market.id, AssetId::POINTS, "SYSTEM", pos)?;

    for (owner, balance) in market.balances.iter() {
        if owner == "SYSTEM" {
            continue;
        }
        let user_points = ensure_account(tx, market.id, AssetId::POINTS, &owner, pos)?;
        create_transfer(tx, event, user_points, pool_points, balance.points)?;
    }

    // Now we again walk all participants and their balances, and we pay out the
    // points that everybody is due.
    for (owner, balance) in market.balances.iter() {
        for oc in &balance.outcomes {
            // TODO: Destroy the outcome shares?

            if oc.1.0 != outcome.0 {
                // This is not the outcome the market resolved to, it's not
                // interesting to us.
                continue;
            }

            // Pay out the participant, from the market's pool account into the
            // owner's global points account. The participant can be the system
            // as well as a user.
            let amount_out = oc.cast(AssetId::POINTS);
            let user_points = ensure_account(tx, MarketId::NONE, AssetId::POINTS, &owner, pos)?;
            create_transfer(tx, event, pool_points, user_points, amount_out)?;

            // Also record the realized profits to make it easier to show the
            // status of a resolved market.
            let amount_in = balance.points;
            db::create_realized_profit(
                tx,
                market.id.0,
                event.0,
                &owner,
                amount_in.0,
                amount_out.0,
            )?;
        }
    }

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

        let x = Amount(1_999_999, AssetId::POINTS);
        assert_eq!(format!("{x}"), "1.999999");
        assert_eq!(format!("{x:.2}"), "2.00");
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

        // Regression test.
        assert_eq!(
            AssetId::POINTS.parse_amount("0.46"),
            Some(Amount(460_000, AssetId::POINTS))
        );
        assert_eq!(
            AssetId::POINTS.parse_amount("0.079965"),
            Some(Amount(79965, AssetId::POINTS))
        );
        assert_eq!(
            AssetId::POINTS.parse_amount("-0.079965"),
            Some(Amount(-79965, AssetId::POINTS))
        );

        // Too many decimals, should refuse.
        assert_eq!(AssetId::POINTS.parse_amount("1.0001234"), None);
    }
}
