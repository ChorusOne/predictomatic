// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2026 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use maud::{Markup, html};

use crate::Response;
use crate::database as db;
use crate::model::{self, Amount, AssetId, Market};
use crate::routes::{Context, Result, index, respond_html, view_header, view_html_head};

fn view_activity_overview(ctx: &Context, trades: &[db::ActivityTrade]) -> Markup {
    html! {
        (view_html_head(ctx.prefix, "Activity — Predict-o-matic"))
        body {
            (view_header(ctx))
            div .main .wider {
                section {
                    h1 { "Activity" }
                    p { "Activity will be show here." }
                    table .nowrap {
                        @for (i, trade) in trades.iter().enumerate() {
                            @if i == 0 || trades[i - 1].market_id != trade.market_id {
                                (view_market_header(ctx, trade))
                            }
                            (view_trade(ctx, trade))
                        }
                    }
                }
                aside { (index::view_main_aside(ctx)) }
            }
        }
    }
}

fn view_market_header(ctx: &Context, trade: &db::ActivityTrade) -> Markup {
    html! {
        tr .market-assets {
            td colspan="6" {
                a href={(ctx.prefix) "/market/" (trade.market_slug)} {
                    (trade.market_title)
                }
            }
        }

    }
}

fn view_trade(ctx: &Context, trade: &db::ActivityTrade) -> Markup {
    let asset = AssetId(trade.asset_id);
    let amount = Amount(trade.amount_bought, asset);
    html! {
        tr {
            td {
                span .num title=(trade.time) {
                    (trade.time[..10])
                    " "
                    (trade.time[11..16])
                }
                ", "
                (ctx.view_email(&trade.user_email))
                " bought "
                (format!("{:.2}", amount))
                " "
                (trade.outcome_label)
            }
        }
    }
}

pub fn handle_activity(tx: &mut db::Transaction, ctx: &Context) -> Result<Response> {
    let limit = 100;
    let before = None;
    let mut activities = Vec::with_capacity(limit as usize);
    for res in db::get_trade_activity_before(tx, limit, before)? {
        activities.push(res?);
    }

    // Sort by date first, but within the same UTC date, sort by market.
    // This clusters trades, so we can emit the header once and list all trades,
    // which makes the page a bit less dense.
    use std::cmp::Reverse;
    activities.sort_by_key(|trade| {
        (
            // The compiler can't infer the right lifetime unfortunately even
            // though all these strings are owned, make a heap-allocated copy
            // then, it's fast enough anyway.
            Reverse(trade.time[..10].to_string()),
            trade.market_id,
            Reverse(trade.event_id),
        )
    });

    Ok(respond_html(view_activity_overview(ctx, &activities)))
}
