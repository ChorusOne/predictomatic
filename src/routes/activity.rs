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

fn view_activity(ctx: &Context) -> Markup {
    html! {
        (view_html_head(ctx.prefix, "Activity — Predict-o-matic"))
        body {
            (view_header(ctx))
            div .main .wider {
                section {
                    h1 { "Activity" }
                    p { "Activity will be show here." }
                }
                aside { (index::view_main_aside(ctx)) }
            }
        }
    }
}

pub fn handle_activity(tx: &mut db::Transaction, ctx: &Context) -> Result<Response> {
    Ok(respond_html(view_activity(ctx)))
}
