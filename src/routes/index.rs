// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use maud::{Markup, html};

use crate::Response;
use crate::database as db;
use crate::model::{self, Market};
use crate::routes::Context;
use crate::routes::market::view_market_stats;
use crate::routes::{respond_html, view_header, view_html_head};

fn view_market_summary(ctx: &Context, market: &Market) -> Markup {
    let dist = market.implied_distribution();
    let ps = dist.ps();

    let teaser = match market.resolution() {
        None => html! { p .teaser { (format!("{:.0}%", ps[0] * 100.0)) } },
        Some(outcome) => html! {
            p
                .teaser .resolved
                title=(format!("Resolved as {}.", outcome.value))
                { (outcome.value) }
        },
    };

    html! {
        div .market {
            h2 {
                a href=(ctx.market_url(market, "")) {
                    (market.title)
                }
            }
            div .summary {
                (teaser)
                (view_market_stats(market, &ps))
            }
        }
    }
}

fn view_index(ctx: &Context, markets: &[Market]) -> Markup {
    let url = |suffix| format!("{}{}", ctx.config_server.prefix, suffix);
    html! {
        (view_html_head("Predict-o-matic"))
        body {
            (view_header(ctx))
            div .main {
                section {
                    p {
                        "Welcome to the prediction market support system. "
                        "Check out one of the markets below to start trading."
                    }
                    @for market in markets {
                        (view_market_summary(ctx, market))
                    }
                }
                aside {
                    h3 { "Account" }
                    p {
                        a href=(url("/assets")) { "Assets" } br;
                        a href=(url("/ledger")) { "Ledger" } br;
                        a href=(url("/ranking")) { "Ranking" }
                    }
                    h3 { "Help" }
                    p {
                        a href=(url("/help")) { "User guide" } br;
                        a href="https://github.com/ChorusOne/predictomatic" { "Source code" }
                    }
                    @if ctx.user.is_admin {
                        h3 { "Administration" }
                        p {
                            a href=(url("/create")) { "Create market" } br;
                            a href=(url("/bonus")) { "Distribute bonus" } br;
                        }
                    }
                }
            }
        }
    }
}

pub fn handle_index(tx: &mut db::Transaction, ctx: &Context) -> db::Result<Response> {
    // TODO: iterate the markets at once rather than getting them by id to save
    // a bit of interop, but it's SQLite so we are not even saving a round-trip,
    // and it's a hackathon so YOLO.
    let market_slugs: Vec<_> = db::get_market_slugs(tx)?.collect();
    let mut markets = Vec::new();
    for res_slug in market_slugs {
        let slug = res_slug?;
        markets.push(model::get_market_by_slug(tx, &slug)?.expect("We know the market exists."));
    }

    // Order markets by descending liquidity.
    markets.sort_by_key(|m| std::cmp::Reverse(m.total_deposited()));

    let body = view_index(ctx, &markets);
    Ok(respond_html(body))
}
