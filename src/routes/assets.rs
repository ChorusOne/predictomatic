// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use maud::{Markup, html};

use crate::Response;
use crate::database as db;
use crate::model::{self, Amount, AssetId, Market};
use crate::routes::Context;
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
            div .main .index {
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
                // TODO: Add the aside from the index page?
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
