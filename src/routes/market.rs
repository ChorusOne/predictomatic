// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use maud::{Markup, html};

use crate::Response;
use crate::database as db;
use crate::model::{self, Amount, AssetId, Balance, Market, RealizedProfit};
use crate::routes::util;
use crate::routes::{
    Context, Result, redirect_see_other, respond_html, view_header, view_html_head,
};

fn get_trade_script() -> Markup {
    // See also how we handle the stylesheet in `mod.rs`.
    maud::PreEscaped(include_str!("../trade.js").to_string())
}

/// A user's position in a given market.
struct MarketPosition<'a> {
    owner: &'a str,
    balance: &'a Balance,

    /// Current market value of the outcome shares held, in points.
    market_value: Amount,

    /// Market value minus amount deposited, in points.
    unrealized_pnl: Amount,
}

pub fn view_market_stats(market: &Market, implied_probabilities: &[f64]) -> Markup {
    let liquidity = market.total_deposited_excluding_system();
    html! {
        table {
            tr {
                td title="Total deposited excluding the system user" { "Liquidity" }
                td .num { (format!("$\u{200a}{liquidity:.2}")) }
            }
            @for (oc, p) in market.outcomes.iter().zip(implied_probabilities) {
                tr {
                    td { (oc.value) }
                    td .num { (format!("$\u{200a}{p:.2}")) }
                }
            }
        }
    }
}
fn view_market_position_aside(market: &Market, position: &MarketPosition) -> Markup {
    html! {
        table {
            @for (oc, pos) in market.outcomes.iter().zip(&position.balance.outcomes) {
                tr {
                    td { (oc.value) }
                    td .num { (format!("{pos:.2}")) }
                }
            }
            // Market value and unrealized UPnL only makes sense if the market
            // is still open.
            @if market.is_open() {
                tr {
                    td { "Market value"}
                    td .num { (format!("$\u{200a}{:.2}", position.market_value))}
                }
                tr {
                    td { "Deposited"}
                    td .num { (format!("$\u{200a}{:.2}", position.balance.points))}
                }
                tr {
                    td { "Unrealized PnL"}
                    td .num { (format!("$\u{200a}{:.2}", position.unrealized_pnl))}
                }
            }
        }
    }
}

fn view_market_participants_open(ctx: &Context, positions: &[MarketPosition]) -> Markup {
    html! {
        table {
            tr {
                th { "Participant" }
                th .num { "Deposit ($)" }
                th .num { "Value ($)" }
                th .num { "UPnL ($)" }
            }
            @for position in positions {
                tr
                    .self[position.owner == ctx.user_email]
                    .system[position.owner == "SYSTEM"]
                {
                    td .owner { (ctx.view_email(position.owner)) }
                    td .num { (format!("{:.2}", position.balance.points)) }
                    td .num { (format!("{:.2}", position.market_value)) }
                    td .num { (format!("{:.2}", position.unrealized_pnl)) }
                }
            }
        }
    }
}

fn view_market_participants_profits(ctx: &Context, positions: &[RealizedProfit]) -> Markup {
    html! {
        table {
            tr {
                th { "Participant" }
                th .num { "Deposit ($)" }
                th .num { "Proceeds ($)" }
                th .num { "Profit ($)" }
            }
            @for position in positions {
                tr
                    .self[position.owner == ctx.user_email]
                    .system[position.owner == "SYSTEM"]
                {
                    td .owner { (ctx.view_email(&position.owner)) }
                    td .num { (format!("{:.2}", position.amount_in)) }
                    td .num { (format!("{:.2}", position.amount_out)) }
                    td .num { (format!("{:.2}", position.amount_out - position.amount_in)) }
                }
            }
        }
    }
}

fn view_resolution(market: &Market) -> Markup {
    html! {
        @if let Some(outcome) = market.resolution() {
            p .resolution { (outcome.value) }
            p { "This market is resolved, you can no longer trade here." }
        }
    }
}

fn view_prediction_binary(
    ctx: &Context,
    market: &Market,
    ps: &[f64],
    user_position: Option<&MarketPosition>,
) -> Markup {
    // The convention is that the first outcome is the positive one.
    let p = ps[0];
    let percentage = format!("{:.1}%", p * 100.0);

    html! {
        div #trade-widget .slider {
            hr;
            span .tmarket {}
            span .tuser {}
            span .percentage .pmarket { (percentage) }
            span .percentage .puser { (percentage) }
            span .knob .disabled {}
        }
        noscript { "You need to enable Javascript to trade." }
        div #trade-offer {
            h3 { "Trade" }
            // The JS replaces the contents of the table when dragging the slider.
            div {
                table {
                    tr {
                        td {
                            "Move the slider to trade."
                        }
                    }
                }
                form name="trade_form" method="post" action=(ctx.market_url(market, "/trade")) {
                    input type="hidden" name="amount_in" value="0";
                    input type="hidden" name="min_out" value="0";
                    input type="hidden" name="asset_in" value="0";
                    input type="hidden" name="asset_out" value="0";
                    button #trade-submit type="submit" disabled { "Trade" }
                }
            }
        }
    }
}

fn view_market_admin(ctx: &Context, market: &Market) -> Markup {
    html! {
        h3 { "Administration" }
        form method="post" {
            @for outcome in &market.outcomes {
                @let url = format!(
                    "{}/market/{}/resolve/{}",
                    ctx.prefix,
                    market.slug,
                    outcome.id.0,
                );
                button .wide type="submit" formaction=(url) {
                    "Resolve " (outcome.value)
                }
            }
        }
    }
}

fn view_market(ctx: &Context, market: &Market) -> Markup {
    let dist = market.implied_distribution();
    let ps = dist.ps();

    // Compute the position of everybody who is participating in this market.
    let mut positions = Vec::new();
    for (owner, balance) in market.balances.iter() {
        let mut market_value = AssetId::POINTS.zero();
        for (oc, p) in balance.outcomes.iter().zip(&ps) {
            market_value = market_value + oc.value_at(*p);
        }
        let unrealized_pnl = market_value - balance.points;
        let pos = MarketPosition {
            owner,
            balance,
            market_value,
            unrealized_pnl,
        };
        positions.push(pos);
    }

    // Sort by descending profit, break ties by sorting by portfolio value.
    positions.sort_by_key(|pos| std::cmp::Reverse((pos.unrealized_pnl, pos.market_value)));

    let our_position = positions.iter().find(|pos| pos.owner == ctx.user_email);
    let system_position = positions
        .iter()
        .find(|pos| pos.owner == "SYSTEM")
        .expect("System always has a position.");

    let deposited = our_position
        .map(|p| p.balance.points)
        .unwrap_or(AssetId::POINTS.zero());

    // We also feed in the serialized current positions into js.
    // TODO: Use serde_json or something instead.
    let balance_user: Vec<String> = match our_position {
        Some(pos) => pos
            .balance
            .outcomes
            .iter()
            .map(|oc| oc.to_string())
            .collect(),
        None => system_position
            .balance
            .outcomes
            .iter()
            .map(|_oc| "0.0".to_string())
            .collect(),
    };
    let balance_system: Vec<String> = system_position
        .balance
        .outcomes
        .iter()
        .map(|oc| oc.to_string())
        .collect();

    html! {
        (view_html_head(ctx.prefix, &format!("{} — Predict-o-matic", market.title)))
        body {
            (view_header(ctx))
            div .main {
                section {
                    h1 { (market.title) }
                    @match market.is_open() {
                        true => (view_prediction_binary(ctx, market, &ps, our_position)),
                        false => (view_resolution(market)),
                    }
                    h2 { "Resolution criteria" }
                    p { (market.description) }
                    h2 { "Participants" }
                    @match market.is_open() {
                        true => (view_market_participants_open(ctx, &positions)),
                        false => (view_market_participants_profits(ctx, &market.profits)),
                    }
                    h2 { "Activity" }
                    p { "In the future I would like to show a log of trades and comments here." }
                }
                aside {
                    (view_market_stats(market, &ps))

                    h3 { "Your balance" }
                    @match our_position {
                        Some(pos) => (view_market_position_aside(market, pos)),
                        None if market.is_open() => { "You are not participating." }
                        None => { "You did not participate." }
                    }

                    @if market.is_open() {
                        h3 { "Bet sizing help "}
                        table #sizing-help-kelly {
                            tr {
                                td { "Belief " (market.outcomes[0].value) }
                                td .num { "—%" }
                            }
                            tr {
                                td { "Kelly bet" }
                                td .num #bet-sizing-c {
                                    "$\u{200a}— of "
                                    (market.outcomes[0].value)
                                }
                            }
                        }
                        div .bet-size-buttons {
                            button
                                onclick="onClickKelly(1/5)"
                                title="Select a trade that trades a fifth of the Kelly bet."
                                { "1/5" }
                            button
                                onclick="onClickKelly(1/4)"
                                title="Select a trade that trades a quarter of the Kelly bet."
                                { "1/4" }
                            button
                                onclick="onClickKelly(1/3)"
                                title="Select a trade that trades a third of the Kelly bet."
                                { "1/3" }
                            button
                                onclick="onClickKelly(1/2)"
                                title="Select a trade that trades half of the Kelly bet."
                                { "1/2" }
                            button
                                onclick="onClickKelly(1)"
                                title="Select the trade that trades the Kelly bet, “Full Kelly”."
                                { "Full" }
                        }
                        div .bet-size-buttons {
                            button
                                #button-neutral
                                onclick="onClickNeutral()"
                                title={
                                    "Select the trade that exits to an outcome-neutral position.\n"
                                    "This effectively realizes profits and losses."
                                  }
                                { "Neutral" }
                        }
                    }

                    @if ctx.is_admin && market.is_open() {
                        (view_market_admin(ctx, market))
                    }
                }
            }
            script {
                @if market.is_open() {
                    "const isOpen = true;\n"
                } @else {
                    "const isOpen = false;\n"
                }
                "const userLiquidPoints = " (ctx.user_points) ";\n"
                "const userDeposited = " (deposited) ";\n"
                "const systemBalance = [" @for b in balance_system { (b) ", " } "];\n"
                "const userBalance = [" @for b in balance_user { (b) ", " } "];\n"
                "const assetIds = [" @for oc in &market.outcomes { (oc.id.0) ", " } "];\n"
                // TODO: Properly serialize the strings as json, Rust's Debug is nto the same!
                // Also it now contains a glaring injection vulnerability.
                "const assetLabels = [" @for oc in &market.outcomes { (maud::PreEscaped(format!("{:?}", oc.value))) ", " } "];\n"
                (get_trade_script())
            }
        }
    }
}

fn get_market_by_slug(
    tx: &mut db::Transaction,
    ctx: &Context,
    market_slug: &str,
) -> Result<Market> {
    match model::get_market_by_slug(tx, market_slug)? {
        None => ctx.not_found("No such market exists."),
        Some(market) => Ok(market),
    }
}

pub fn handle_market(
    tx: &mut db::Transaction,
    ctx: &Context,
    market_slug: &str,
) -> Result<Response> {
    let market = get_market_by_slug(tx, ctx, market_slug)?;
    let body = view_market(ctx, &market);
    Ok(respond_html(body))
}

pub fn handle_deposit(
    tx: &mut db::Transaction,
    ctx: &Context,
    market_slug: &str,
    body: &str,
) -> Result<Response> {
    let market = get_market_by_slug(tx, ctx, market_slug)?;
    let amount = util::parse_form_amount(ctx, body)?;
    model::create_deposit(tx, &market, amount, ctx.user_email)?;
    Ok(redirect_see_other(ctx.market_url(&market, "")))
}

pub fn handle_trade(
    tx: &mut db::Transaction,
    ctx: &Context,
    market_slug: &str,
    body: &str,
) -> Result<Response> {
    use std::str::FromStr;

    let market = get_market_by_slug(tx, ctx, market_slug)?;

    let mut amount_in_str = None;
    let mut min_out_str = None;
    let mut asset_in = AssetId::POINTS;
    let mut asset_out = AssetId::POINTS;

    for (key, value) in form_urlencoded::parse(body.as_bytes()) {
        match key.as_ref() {
            // TODO: We can avoid the to_string, but this code is ugly enough as it is.
            "amount_in" => amount_in_str = Some(value.to_string()),
            "min_out" => min_out_str = Some(value.to_string()),
            "asset_in" => match i64::from_str(value.as_ref()) {
                Ok(n) => asset_in = AssetId(n),
                Err(..) => return ctx.bad_request("Invalid asset id for asset_in."),
            },
            "asset_out" => match i64::from_str(value.as_ref()) {
                Ok(n) => asset_out = AssetId(n),
                Err(..) => return ctx.bad_request("Invalid asset id for asset_out."),
            },
            _ => return ctx.bad_request("Unexpected form data."),
        }
    }

    let mut amount_in = match amount_in_str {
        Some(v) if asset_in != AssetId::POINTS => match asset_in.parse_amount(&v) {
            Some(n) => n,
            None => {
                return ctx.bad_request("Invalid amount_in amount or in asset id absent.");
            }
        },
        _ => {
            return ctx.bad_request("Missing amount_in, asset id, or invalid asset id.");
        }
    };
    let min_out = match min_out_str {
        Some(v) if asset_out != AssetId::POINTS => match asset_out.parse_amount(&v) {
            Some(n) => n,
            None => {
                return ctx.bad_request("Invalid min_out amount or in asset id absent.");
            }
        },
        _ => {
            return ctx.bad_request("Missing min_out amount, asset id, or invalid asset id.");
        }
    };

    if amount_in.0 <= 0 || min_out.0 <= 0 {
        return ctx.bad_request(format!(
            "Amount amount_in and min_out must be greater than zero, but got {} and {}.",
            amount_in, min_out,
        ));
    }

    // Due to the way the frontend computes amount_in, it may overestimate the
    // amount it wants to trade by a slight amount, and that then causes an
    // constraint violation due to trying to trade more than the available
    // balance. For small differences we can fix that by limiting amount_in to
    // the amount we have available to spend.
    if let Some(user_balance) = market.balances.get(ctx.user_email) {
        for b in &user_balance.outcomes {
            if b.1 == amount_in.1 {
                amount_in = std::cmp::min(amount_in, *b);
                break;
            }
        }
    }

    let amount_out = match market.trade(amount_in, min_out) {
        Some(amount) => amount,
        None => {
            return ctx.conflict(
                "Order failed. \
                This can happen if somebody traded just before you, \
                and the order exceeded your slippage tolerance. \
                Go back, refresh the page, and try again.",
            );
        }
    };

    println!(
        "Trading in market {}: {:?}:{amount_in} -> {:?}:{amount_out}",
        market.slug, asset_in, asset_out
    );
    model::create_trade(tx, &market, amount_in, amount_out, ctx.user_email)?;

    Ok(redirect_see_other(ctx.market_url(&market, "")))
}

pub fn handle_resolve(
    tx: &mut db::Transaction,
    ctx: &Context,
    market_slug: &str,
    outcome_str: &str,
) -> Result<Response> {
    use std::str::FromStr;

    let market = get_market_by_slug(tx, ctx, market_slug)?;

    if !ctx.is_admin {
        return ctx.forbidden("Only admins are allowed to resolve markets.");
    }

    let outcome = match i64::from_str(outcome_str) {
        Err(..) => return ctx.bad_request("Invalid outcome id."),
        Ok(i) => match market.outcomes.iter().find(|oc| oc.id.0 == i) {
            None => return ctx.bad_request("That outcome does not exist in this market."),
            Some(oc) => oc,
        },
    };

    model::create_resolution(tx, &market, outcome.id)?;

    Ok(redirect_see_other(ctx.market_url(&market, "")))
}
