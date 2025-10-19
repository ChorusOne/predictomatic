// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use std::collections::HashMap;

use maud::{Markup, html};

use crate::Response;
use crate::database as db;
use crate::model::{self, Amount, AssetId, Market};
use crate::routes::Context;
use crate::routes::index;
use crate::routes::{respond_html, view_header, view_html_head};

struct MarketAssets<'a> {
    market: &'a Market,

    /// Summary of the outcome shares the user owns.
    assets: Vec<UserAsset<'a>>,

    /// Market value of the user's outcome shares combined.
    total_value: Amount,
}

struct UserAsset<'a> {
    /// Outcome label.
    label: &'a str,

    /// Number of outcome shares the user owns.
    amount: Amount,

    /// Price per share, in points (not in micros!).
    price: f64,

    /// Market value of the outcome shares, in points.
    value: Amount,
}

fn view_market_assets(ctx: &Context, market: &MarketAssets) -> Markup {
    html! {
        tr .market-assets {
            td colspan="3" {
                a href=(ctx.market_url(market.market, "")) {
                    (market.market.title)
                }
            }
            td .num {
                (format!("{:.2}", market.total_value))
            }
        }
        @for asset in &market.assets {
            tr {
                td { (asset.label) }
                td .num { (format!("{:.2}", asset.amount)) }
                td .num { (format!("{:.2}", asset.price)) }
                td .num { (format!("{:.2}", asset.value)) }
            }
        }
    }
}

fn view_assets_overview(ctx: &Context, markets: &[MarketAssets]) -> Markup {
    let mut total_illiquid = AssetId::POINTS.zero();
    for market in markets {
        total_illiquid = total_illiquid + market.total_value;
    }
    let net_worth = total_illiquid + ctx.user_points;

    html! {
        (view_html_head("Predict-o-matic"))
        body {
            (view_header(ctx))
            div .main .wider {
                section {
                    // TODO: Would be nice to render a pie chart with top assets
                    // here.
                    table .wide {
                        tr {
                            th { "Asset" }
                            th .num .w5 { "Amount" }
                            th .num .w5 { "Price ($)" }
                            th .num .w5 { "Value ($)" }
                        }
                        tr {
                            td { "Liquid points" }
                            td .num { (format!("{:.2}", ctx.user_points)) }
                            td .num { "1.00" }
                            td .num { (format!("{:.2}", ctx.user_points)) }
                        }
                        tr {
                            td colspan="3" { "Illiquid outcome shares" }
                            td .num { (format!("{:.2}", total_illiquid)) }
                        }
                        tr {
                            td colspan="3" { "Total" }
                            td .num .strong { (format!("{:.2}", net_worth)) }
                        }
                        @for market in markets {
                            (view_market_assets(ctx, market))
                        }
                    }
                    @if markets.is_empty() {
                        p {
                            "You don’t own any outcome shares. "
                            "Go bet in some markets!"
                        }
                    }
                }
                aside .rule {
                    (index::view_main_aside(ctx))
                }
            }
        }
    }
}

pub fn handle_assets_overview(tx: &mut db::Transaction, ctx: &Context) -> db::Result<Response> {
    // TODO: See also the note in index.rs about the inefficiency to iterate all
    // markets. On top of that, for the asset overview we don't need all markets,
    // and we don't need all accounts of all users! But for now we load them
    // anyway because it's a quick way to get started, if it ever becomes too
    // slow then we can limit the accounts we load from the database.
    let market_slugs: Vec<_> = db::get_market_slugs(tx)?.collect();
    let mut markets = Vec::new();
    for res_slug in market_slugs {
        let slug = res_slug?;
        markets.push(model::get_market_by_slug(tx, &slug)?.expect("We know the market exists."));
    }

    let mut market_assets = Vec::new();
    for market in &markets {
        if !market.is_open() {
            continue;
        }
        let balance = match market.balances.get(ctx.user_email) {
            Some(b) => b,
            None => continue,
        };

        let dist = market.implied_distribution();

        let mut assets = Vec::with_capacity(market.outcomes.len());
        let mut total_value = AssetId::POINTS.zero();

        for ((oc, b), p) in market.outcomes.iter().zip(&balance.outcomes).zip(dist.ps()) {
            let value = b.value_at(p);
            let asset = UserAsset {
                label: &oc.value,
                amount: *b,
                price: p,
                value,
            };
            assets.push(asset);
            total_value = total_value + value;
        }

        // List outcome shares by descending position size.
        assets.sort_by_key(|a| std::cmp::Reverse(a.amount.0));

        market_assets.push(MarketAssets {
            market,
            assets,
            total_value,
        })
    }

    // Order markets in which we own assets by descending market value.
    market_assets.sort_by_key(|m| std::cmp::Reverse(m.total_value));

    let body = view_assets_overview(ctx, &market_assets);
    Ok(respond_html(body))
}

struct UserNetWorth {
    /// Liquid points in the user's global points account.
    liquid: Amount,

    /// Market value of the user's outcome shares.
    illiquid: Amount,

    /// Sum of `liquid` and `illiquid`.
    total: Amount,
}

impl UserNetWorth {
    fn new() -> Self {
        Self {
            liquid: AssetId::POINTS.zero(),
            illiquid: AssetId::POINTS.zero(),
            total: AssetId::POINTS.zero(),
        }
    }
}

fn view_net_worth_list(ctx: &Context, users_net_worth: &[(String, UserNetWorth)]) -> Markup {
    html! {
        (view_html_head("Predict-o-matic"))
        body {
            (view_header(ctx))
            div .main .wider {
                section {
                    table .wide {
                        tr {
                            th .w2 { "№" }
                            th { "Participant" }
                            th .num .w5 { "Liquid Assets ($)" }
                            th .num .w5 { "Illiquid Assets ($)" }
                            th .num .w5 { "Net" br; "Worth ($)" }
                        }
                        @for (i, (owner, net_worth)) in users_net_worth.iter().enumerate() {
                            tr .self[owner == ctx.user_email] {
                                // TODO: View self.
                                @let rank = i + 1;
                                td { (rank) }
                                td .owner { (ctx.view_email(owner)) }
                                td .num { (format!("{:.0}", net_worth.liquid)) }
                                td .num { (format!("{:.0}", net_worth.illiquid)) }
                                td .num { (format!("{:.0}", net_worth.total)) }
                            }
                        }
                    }
                }
                aside .rule {
                    (index::view_main_aside(ctx))
                }
            }
        }
    }
}

pub fn handle_leaderboard(tx: &mut db::Transaction, ctx: &Context) -> db::Result<Response> {
    // See also the note in `handle_assets_overview`.
    // TODO: Add at least a way to only query the open markets.
    let market_slugs: Vec<_> = db::get_market_slugs(tx)?.collect();
    let mut markets = Vec::new();
    for res_slug in market_slugs {
        let slug = res_slug?;
        markets.push(model::get_market_by_slug(tx, &slug)?.expect("We know the market exists."));
    }

    // We are going to map owners (emails) to their net worth.
    let mut users: HashMap<String, UserNetWorth> = HashMap::new();

    for market in markets.into_iter() {
        // For illiquid assets, only the open markets are relevant.
        if !market.is_open() {
            continue;
        }

        let dist = market.implied_distribution();
        let ps = dist.ps();

        for (owner, balance) in market.balances.into_iter() {
            let net_worth = users.entry(owner).or_insert_with(UserNetWorth::new);
            for (b, p) in balance.outcomes.iter().zip(&ps) {
                net_worth.illiquid = net_worth.illiquid + b.value_at(*p);
            }
        }
    }

    // Convert the hash map to a list so we can sort it. While at it we compute
    // the totals, which we can do now that both components are known.
    let mut users_net_worth = Vec::with_capacity(users.len());
    for (owner, mut net_worth) in users.into_iter() {
        net_worth.total = net_worth.liquid + net_worth.illiquid;
        users_net_worth.push((owner, net_worth));
    }

    users_net_worth.sort_by_key(|(_u, nw)| std::cmp::Reverse(nw.total));

    let body = view_net_worth_list(ctx, &users_net_worth);
    Ok(respond_html(body))
}
