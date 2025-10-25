// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use maud::{Markup, html};

use crate::Response;
use crate::routes::{Context, Result, index, respond_html, view_header, view_html_head};

fn view_help(ctx: &Context) -> Markup {
    html! {
        (view_html_head(ctx.prefix, "Predict-o-matic"))
        body {
            (view_header(ctx))
            div .main .wider {
                section {
                    h1 { "User manual" }
                    // TODO: Extract into some markdown doc.
                    p {
                        "Predict-o-matic facilitates prediction markets.
                        Prediction markets are a tool for aggregating
                        information about future events.
                        This manual provides a short introduction to prediction markets,
                        and how they work in Predict-o-matic."
                    }
                    p { "This manual is a work in progress." }
                    h2 { "Introduction" }
                    p {
                        "You have points. You have "
                        (format!("{:.2}", ctx.user_points))
                        " of them right now.
                        You can also see this in the top-right corner of the screen.
                        You can deposit points into " em { "markets" } ".
                        When you do that, you purchase " em { "outcome shares" }
                        " in that market. For example, you deposit $\u{200a}10
                        into a market “Will it rain next Tuesday?”,
                        and you receive 10 Yes and 10 No shares in return.
                        When the market resolves,
                        if it did rain, the Yes shares pay out $1 each,
                        and the No shares become worthless.
                        If it did not rain, the No shares pay out $1,
                        and the Yes shares are worthless.
                        Because the two outcomes are the only possibilities,
                        1 Yes + 1 No is always worth $1."
                    }
                    p {
                        "Now that you have some outcome shares, you can trade them.
                        You trade by exchanging shares for other shares,
                        not by trading them for points directly.
                        For example, you buy 5 Yes shares,
                        and you pay for that with 10 No shares.
                        The ratio of No:Yes corresponds to the " em { "odds" }
                        " of a positive outcome.
                        Odds of 10:5 mean a probability of 10/15,
                        so about 67% probability that ‘Yes’ will happen,
                        and 33% that ‘No’ will happen.
                        In other words, the average price you paid was $0.67 per
                        Yes share."
                    }
                    p {
                        "When you trade, you always trade against an automated
                        market maker that is managed by the system. The market
                        maker ensures that you can always trade, and it provides
                        a way to subsidize the market. Prediction markets are
                        zero-sum. If you are going to make money here, "
                        em { "somebody" } " has to be losing money. When you are
                        the only participant in a market, or when all
                        participants bet in the same direction, it’s the system
                        that takes the other side of the bet."
                    }
                }
                aside { (index::view_main_aside(ctx)) }
            }
        }
    }
}

pub fn handle_help(ctx: &Context) -> Result<Response> {
    Ok(respond_html(view_help(ctx)))
}
