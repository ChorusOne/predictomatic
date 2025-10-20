// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

mod assets;
mod help;
mod index;
mod leaderboard;
mod ledger;
mod market;

use maud::{DOCTYPE, Markup, html};
use tiny_http::Header;

use crate::Response;
use crate::config::{AppConfig, ServerConfig};
use crate::database as db;
use crate::model::{self, Amount, Market};

pub struct Context<'a> {
    config: &'a AppConfig,

    /// Prefix as which the entire app is being served, from the server config.
    prefix: &'a str,

    /// Email address of the currently logged in user.
    user_email: &'a str,

    /// Whether the current user is an admin.
    is_admin: bool,

    /// The user's liquid balance (in their global points account).
    user_points: Amount,
}

impl<'a> Context<'a> {
    pub fn new(
        config_app: &'a AppConfig,
        config_server: &'a ServerConfig,
        user_email: &'a str,
        tx: &mut db::Transaction,
    ) -> db::Result<Context<'a>> {
        let user_points_account = model::ensure_points_account(tx, config_app, user_email)?;
        let user_points = model::get_account_balance(tx, user_points_account)?;
        let ctx = Context {
            config: config_app,
            prefix: &config_server.prefix,
            user_email,
            is_admin: config_app.admin_email == user_email,
            user_points,
        };
        Ok(ctx)
    }

    pub fn market_url(&self, market: &Market, suffix: &str) -> String {
        format!("{}/market/{}{}", self.prefix, market.slug, suffix)
    }

    fn view_email<'b>(&self, email: &'b str) -> &'b str {
        match email.strip_suffix(&self.config.email_suffix) {
            Some(stripped) => stripped,
            None => email,
        }
    }

    /// Serve a bad request response.
    ///
    /// This is just a wrapper, but we add it here because this error is used a
    /// lot, and it's annoying to write it out by hand all the time.
    fn bad_request<R: Into<String>>(&self, reason: R) -> db::Result<Response> {
        Ok(respond_error(self.prefix, reason).with_status_code(400))
    }

    fn forbidden<R: Into<String>>(&self, reason: R) -> db::Result<Response> {
        Ok(respond_error(self.prefix, reason).with_status_code(403))
    }

    fn not_found<R: Into<String>>(&self, reason: R) -> db::Result<Response> {
        Ok(respond_error(self.prefix, reason).with_status_code(404))
    }

    fn conflict<R: Into<String>>(&self, reason: R) -> db::Result<Response> {
        Ok(respond_error(self.prefix, reason).with_status_code(409))
    }
}

fn respond_html(markup: Markup) -> Response {
    Response::from_string(markup.into_string()).with_header(
        Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap(),
    )
}

fn respond_svg(svg: &str) -> Response {
    let header_content_type =
        Header::from_bytes(&b"Content-Type"[..], &b"image/svg+xml; charset=utf-8"[..]).unwrap();

    // Allow the browser to cache for 48 hours.
    let header_cache_control =
        Header::from_bytes(&b"Cache-Control"[..], &b"max-age=172800"[..]).unwrap();

    Response::from_string(svg)
        .with_header(header_content_type)
        .with_header(header_cache_control)
}

fn respond_error<R: Into<String>>(server_prefix: &str, reason: R) -> Response {
    let page = html! {
        (view_html_head(server_prefix, "Predict-o-matic Error"))
        body {
            div .main-error {
                h1 { "D’oh!" }
                p { (reason.into()) }
            }
        }
    };
    respond_html(page)
}

pub fn internal_error<R: Into<String>>(prefix: &str, reason: R) -> Response {
    respond_error(prefix, reason).with_status_code(500)
}

pub fn service_unavailable<R: Into<String>>(prefix: &str, reason: R) -> Response {
    respond_error(prefix, reason).with_status_code(503)
}

pub fn redirect_see_other<R: AsRef<[u8]>>(location: R) -> Response {
    Response::from_string("")
        .with_status_code(303)
        .with_header(Header::from_bytes(&b"Location"[..], location.as_ref()).unwrap())
}

/// Render the standard header that is the same across all pages.
///
/// Takes the site-wide prefix from the server config. We don't pass in the
/// `Context` here because error pages also need this head, and we might
/// encounter an error before we can construct the full context.
fn view_html_head(server_prefix: &str, page_title: &str) -> Markup {
    html! {
        (DOCTYPE)
        head {
            meta charset="utf-8";
            link rel="preconnect" href="https://fonts.googleapis.com";
            link rel="preconnect" href="https://fonts.gstatic.com" crossorigin;
            link href="https://fonts.googleapis.com/css2?family=Atkinson+Hyperlegible+Next:ital,wght@0,200..800;1,200..800&display=swap" rel="stylesheet";
            link rel="icon" href={(server_prefix) "/icon.svg"} sizes="any" type="image/svg+xml";
            meta name="viewport" content="width=device-width, initial-scale=1";
            title { (page_title) }
            style { (get_stylesheet()) }
        }
    }
}

fn get_stylesheet() -> Markup {
    // In the past, we had different functions for this in debug and release
    // mode, where in release we embed the data in the binary, and in debug we
    // load it from disk, so you can edit it and preview without rebuilding.
    // But it turns out, recompiling is fast enough, you can watch files and
    // recompile on change, so we can simplify this to just always include.
    let data = include_str!("../style.css");
    maud::PreEscaped(data.to_string())
}

fn get_favicon() -> &'static str {
    include_str!("../../assets/favicon.svg")
}

fn view_header(ctx: &Context) -> Markup {
    let root_url = match ctx.prefix {
        "" => "/",
        pf => pf,
    };
    html! {
        nav {
            h1 {
                a href=(root_url) { "Predict-o-matic" }
            }
            " "
            span .balance {
                (format!("$\u{200a}{:.2}", ctx.user_points))
            }
            " "
            span .user {
                (ctx.user_email)
            }
        }
    }
}

pub fn handle_get(tx: &mut db::Transaction, ctx: &Context, path: &[&str]) -> db::Result<Response> {
    match path {
        [] | [""] => index::handle_index(tx, ctx),
        ["assets"] => assets::handle_assets_overview(tx, ctx),
        ["icon.svg"] => Ok(respond_svg(get_favicon())),
        ["help"] => help::handle_help(ctx),
        ["leaderboard"] => leaderboard::handle_leaderboard(tx, ctx),
        ["ledger"] => ledger::handle_ledger(ctx),
        ["market", market_slug] => market::handle_market(tx, ctx, market_slug),
        _ => ctx.not_found("Not found."),
    }
}

pub fn handle_post(
    tx: &mut db::Transaction,
    ctx: &Context,
    path: &[&str],
    body: &str,
) -> db::Result<Response> {
    match path {
        ["market", market_slug, "deposit"] => market::handle_deposit(tx, ctx, market_slug, body),
        ["market", market_slug, "trade"] => market::handle_trade(tx, ctx, market_slug, body),
        ["market", market_slug, "resolve", outcome] => {
            market::handle_resolve(tx, ctx, market_slug, outcome)
        }
        _ => ctx.not_found("Not found."),
    }
}
