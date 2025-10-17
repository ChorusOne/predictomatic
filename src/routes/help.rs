// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use maud::{Markup, html};

use crate::Response;
use crate::database as db;
use crate::routes::Context;
use crate::routes::{respond_html, view_header, view_html_head};

fn view_help(ctx: &Context) -> Markup {
    html! {
        (view_html_head("Predict-o-matic"))
        body {
            (view_header(ctx))
            div .main {
                section {
                    p {
                        "I need to write a user guide here."
                    }
                }
                aside {
                    h3 { "Contents" }
                    p {
                        "Getting started" br;
                        "Accounts" br;
                    }
                }
            }
        }
    }
}

pub fn handle_help(ctx: &Context) -> db::Result<Response> {
    Ok(respond_html(view_help(ctx)))
}
