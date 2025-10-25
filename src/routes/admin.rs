// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use maud::{Markup, html};

use crate::Response;
use crate::routes::{Context, Result, index, respond_html, view_header, view_html_head};

fn view_bonus_page(ctx: &Context) -> Markup {
    html! {
        (view_html_head(ctx.prefix, "Predict-o-matic"))
        body {
            (view_header(ctx))
            div .main .wider {
                section {
                    h1 { "Distribute bonus" }
                }
                aside { (index::view_main_aside(ctx)) }
            }
        }
    }
}

pub fn handle_bonus_page(ctx: &Context) -> Result<Response> {
    ctx.ensure_admin()?;
    Ok(respond_html(view_bonus_page(ctx)))
}
