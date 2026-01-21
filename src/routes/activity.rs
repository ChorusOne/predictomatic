// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2026 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use maud::{Markup, html};

use crate::Response;
use crate::database as db;
use crate::model::TradeActivity;
use crate::routes::{Context, Result, index, respond_html, view_header, view_html_head};

fn view_activity_overview(ctx: &Context, trades: &[db::TradeActivity], per_page: usize) -> Markup {
    html! {
        (view_html_head(ctx.prefix, "Activity — Predict-o-matic"))
        body {
            (view_header(ctx))
            div .main .wider {
                section {
                    h1 { "Activity" }
                    p { "This page shows a reverse-chronological overview of trades across all markets." }
                    table .nowrap {
                        @for (i, trade) in trades.iter().enumerate().take(per_page) {
                            @if i == 0 || trades[i - 1].market_id != trade.market_id {
                                (view_market_header(ctx, trade))
                            }
                            (view_trade(ctx, trade.into()))
                        }
                    }
                    @if let Some(last) = trades.get(per_page) {
                        p {
                            a href={(ctx.prefix) "/activity/" (last.event_id)} {
                                "Older activity →"
                            }
                        }
                    }
                }
                aside { (index::view_main_aside(ctx)) }
            }
        }
    }
}

fn view_market_header(ctx: &Context, trade: &db::TradeActivity) -> Markup {
    html! {
        tr .section-header {
            td colspan="6" {
                a href={(ctx.prefix) "/market/" (trade.market_slug)} {
                    (trade.market_title)
                }
            }
        }
    }
}

pub fn view_trade(ctx: &Context, trade: TradeActivity) -> Markup {
    html! {
        // Give rows an id so you can link to a particular one if needed.
        tr id={"event-" (trade.event_id.0)} {
            td {
                span .num title=(trade.created_at) {
                    (trade.created_at[..10])
                    " "
                    (trade.created_at[11..16])
                }
                ", "
                (ctx.view_email(trade.user_email))
                " bought "
                (format!("{:.2}", trade.amount_bought))
                " "
                (trade.outcome_label)
            }
        }
    }
}

pub fn handle_activity_overview(
    tx: &mut db::Transaction,
    ctx: &Context,
    max_event_id_str: Option<&str>,
) -> Result<Response> {
    use std::str::FromStr;
    let max_event_id = match max_event_id_str {
        None => None,
        Some(s) => match i64::from_str(s) {
            Ok(i) => Some(i),
            Err(..) => return ctx.bad_request("Invalid event id for activity overview."),
        },
    };

    // We try to select one more than what we display. This way we know if there
    // should be a next page, and we also know the id of the first event on the
    // next page.
    let per_page = 100;
    let mut activities = Vec::with_capacity(per_page + 1);
    for res in db::get_trade_activity_until(tx, (per_page + 1) as i64, max_event_id)? {
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
            Reverse(trade.created_at[..10].to_string()),
            trade.market_id,
            Reverse(trade.event_id),
        )
    });

    Ok(respond_html(view_activity_overview(
        ctx,
        &activities,
        per_page,
    )))
}
