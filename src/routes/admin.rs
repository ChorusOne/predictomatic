// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use maud::{Markup, html};

use crate::Response;
use crate::database as db;
use crate::model::{self};
use crate::routes::util;
use crate::routes::{
    Context, Result, index, redirect_see_other, respond_html, view_header, view_html_head,
};

fn view_bonus_page(ctx: &Context) -> Markup {
    html! {
        (view_html_head(ctx.prefix, "Predict-o-matic"))
        body {
            (view_header(ctx))
            div .main .wider {
                section {
                    h1 { "Distribute bonus" }
                    p {
                        "A bonus is a one-time payment of additional points. "
                        "Every existing user will receive the bonus. "
                        "A bonus does not affect future sign-ups."
                    }
                    p {
                        form method="post" action={ (ctx.prefix) "/bonus/create" } {
                            label {
                                "Amount "
                                input #input-bonus-amount name="amount" value="$10.00";
                            }
                            " "
                            button #button-distribute type="submit" { "Distribute" }
                        }
                    }
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

pub fn handle_bonus_create(
    tx: &mut db::Transaction,
    ctx: &Context,
    body: &str,
) -> Result<Response> {
    ctx.ensure_admin()?;
    let amount = util::parse_form_amount(ctx, body)?;
    model::create_bonus(tx, amount, ctx.user_email)?;
    Ok(redirect_see_other(format!("{}/bonus", ctx.prefix)))
}
