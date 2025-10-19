// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use maud::{Markup, html};

use crate::Response;
use crate::database as db;
use crate::routes::{Context, index, respond_html, view_header, view_html_head};

fn view_ledger(ctx: &Context) -> Markup {
    html! {
        (view_html_head(ctx.prefix, "Ledger — Predict-o-matic"))
        body {
            (view_header(ctx))
            div .main .wider {
                section {
                    h1 { "Ledger" }
                    p {
                        "I would like to show the log of all your trades here,
                        and other balance changes to your accounts.
                        Unfortunately this is not yet implemented. Rest assured
                        though, your full transaction history is stored safely
                        in the database."
                    }
                }
                aside { (index::view_main_aside(ctx)) }
            }
        }
    }
}

pub fn handle_ledger(ctx: &Context) -> db::Result<Response> {
    Ok(respond_html(view_ledger(ctx)))
}
