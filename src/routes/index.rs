// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use maud::{Markup, html};

use crate::Response;
use crate::database as db;
use crate::model::{self, Market};
use crate::routes::market::view_market_stats;
use crate::routes::{Context, Result, respond_html, view_header, view_html_head};

fn view_market_summary(ctx: &Context, market: &Market) -> Markup {
    let dist = market.implied_distribution();
    let ps = dist.ps();

    let teaser = match market.resolution() {
        None => html! {
            div .teaser {
                span .big { (format!("{:.0}%", ps[0] * 100.0)) }
                span .below { (market.outcomes[0].value) }
            }
        },
        Some(outcome) => html! {
            div
                .teaser .resolved
                title=(format!("Resolved as {}.", outcome.value))
            {
                span .big { (outcome.value) }
                span .below { "Resolved" }
            }
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

/// View the main sidebar, used on the index page and some others.
pub fn view_main_aside(ctx: &Context) -> Markup {
    let url = |suffix| format!("{}{}", ctx.prefix, suffix);
    html! {
        h3 { "Account" }
        p {
            a href=(url("/assets")) { "Assets" } br;
            a href=(url("/ledger")) { "Ledger" } br;
            a href=(url("/leaderboard")) { "Leaderboard" }
        }
        h3 { "Help" }
        p {
            a href=(url("/help")) { "User manual" } br;
            a href="https://github.com/ChorusOne/predictomatic" { "Source code" }
        }
        @if ctx.is_admin {
            h3 { "Administration" }
            p {
                a href=(url("/create")) { "Create market" } br;
                a href=(url("/bonus")) { "Distribute bonus" } br;
            }
        }
    }
}

fn view_index(ctx: &Context, markets: &[Market]) -> Markup {
    html! {
        (view_html_head(ctx.prefix, "Predict-o-matic"))
        body {
            (view_header(ctx))
            div .main .wider {
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
                    (view_main_aside(ctx))
                }
            }
        }
    }
}

pub fn handle_index(tx: &mut db::Transaction, ctx: &Context) -> Result<Response> {
    // TODO: iterate the markets at once rather than getting them by id to save
    // a bit of interop, but it's SQLite so we are not even saving a round-trip,
    // and it's a hackathon so YOLO.
    let market_slugs: Vec<_> = db::get_market_slugs(tx)?.collect();
    let mut markets = Vec::new();
    for res_slug in market_slugs {
        let slug = res_slug?;
        markets.push(model::get_market_by_slug(tx, &slug)?.expect("We know the market exists."));
    }

    // Order markets by ranking score.
    markets.sort_by_key(|m| m.index_rank());

    let body = view_index(ctx, &markets);
    Ok(respond_html(body))
}
