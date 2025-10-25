// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use std::collections::HashMap;

use maud::{Markup, html};

use crate::Response;
use crate::database as db;
use crate::model::{self, Amount, AssetId};
use crate::routes::index;
use crate::routes::{Context, Result, respond_html, view_header, view_html_head};

struct UserNetWorth {
    /// Liquid points in the user's global points account.
    liquid: Amount,

    /// Market value of the user's outcome shares.
    illiquid: Amount,

    /// Sum of `liquid` and `illiquid`.
    total: Amount,
}

fn view_net_worth_list(ctx: &Context, users_net_worth: &[(String, UserNetWorth)]) -> Markup {
    html! {
        (view_html_head(ctx.prefix, "Leaderboard — Predict-o-matic"))
        body {
            (view_header(ctx))
            div .main .wider {
                section {
                    h1 { "Leaderboard" }
                    p {
                        "A prediction market is a zero-sum game. "
                        "Money flows from those who make bad predictions "
                        "to those who make good predictions. "
                        "Therefore, net worth is some indicator "
                        "of who is making good predictions."
                    }
                    table {
                        tr {
                            th .w2 { "№" }
                            th { "Participant" }
                            th .num .w5 { "Liquid Assets ($)" }
                            th .num .w5 { "Illiquid Assets ($)" }
                            th .num .w5 { "Net" br; "Worth ($)" }
                        }
                        @for (i, (owner, net_worth)) in users_net_worth.iter().enumerate() {
                            tr
                                .self[owner == ctx.user_email]
                                .system[owner == "SYSTEM"]
                            {
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
                aside {
                    (index::view_main_aside(ctx))
                }
            }
        }
    }
}

pub fn handle_leaderboard(tx: &mut db::Transaction, ctx: &Context) -> Result<Response> {
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

    for res_account in db::get_points_accounts(tx)? {
        let account = res_account?;
        users.insert(
            account.owner,
            UserNetWorth {
                liquid: Amount(account.balance, AssetId::POINTS),
                illiquid: AssetId::POINTS.zero(),
                total: AssetId::POINTS.zero(),
            },
        );
    }

    // TODO: The system's points account is also where all sign-on bonuses get
    // paid from, so it goes very very negative. On the one hand, that's nice,
    // because the sum of the net worths is always 0. On the other hand, it says
    // little about the market maker performance and more about the number of
    // users who joined. We might instead count the system's liquid assets as
    // those locked up in markets.

    for market in markets.into_iter() {
        // For illiquid assets, only the open markets are relevant.
        if !market.is_open() {
            continue;
        }

        let dist = market.implied_distribution();
        let ps = dist.ps();

        for (owner, balance) in market.balances.into_iter() {
            let net_worth = users
                .get_mut(&owner)
                .expect("Users who traded have a points account, so they are in the map already.");
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
