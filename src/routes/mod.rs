// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

pub mod index;
pub mod market;

use maud::{DOCTYPE, Markup, html};
use tiny_http::Header;

use crate::config::Config;
use crate::database as db;
use crate::model::{self, Amount, Market};
use crate::{Response, User};

pub struct Context<'a> {
    config: &'a Config,
    user: &'a User,
    user_points: Amount,
}

impl<'a> Context<'a> {
    pub fn new(
        config: &'a Config,
        user: &'a User,
        tx: &mut db::Transaction,
    ) -> db::Result<Context<'a>> {
        let user_points_account = model::ensure_points_account(tx, &config.app, &user.email)?;
        let user_points = model::get_account_balance(tx, user_points_account)?;
        let ctx = Context {
            config,
            user,
            user_points,
        };
        Ok(ctx)
    }

    pub fn market_url(&self, market: &Market, suffix: &str) -> String {
        format!(
            "{}/market/{}{}",
            self.config.server.prefix, market.slug, suffix
        )
    }

    fn view_email<'b>(&self, email: &'b str) -> &'b str {
        match email.strip_suffix(&self.config.app.email_suffix) {
            Some(stripped) => stripped,
            None => email,
        }
    }
}

fn respond_html(markup: Markup) -> Response {
    Response::from_string(markup.into_string()).with_header(
        Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap(),
    )
}

fn respond_error<R: Into<String>>(reason: R) -> Response {
    let page = html! {
        (view_html_head("Predict-o-matic Error"))
        body {
            div .main-error {
                h1 { "D’oh!" }
                p { (reason.into()) }
            }
        }
    };
    respond_html(page)
}

pub fn bad_request<R: Into<String>>(reason: R) -> Response {
    respond_error(reason).with_status_code(400)
}

pub fn not_found<R: Into<String>>(reason: R) -> Response {
    respond_error(reason).with_status_code(404)
}

fn conflict<R: Into<String>>(reason: R) -> Response {
    respond_error(reason).with_status_code(409)
}

fn forbidden<R: Into<String>>(reason: R) -> Response {
    respond_error(reason).with_status_code(403)
}

pub fn internal_error<R: Into<String>>(reason: R) -> Response {
    respond_error(reason).with_status_code(500)
}

pub fn service_unavailable<R: Into<String>>(reason: R) -> Response {
    respond_error(reason).with_status_code(503)
}

fn redirect_see_other<R: AsRef<[u8]>>(location: R) -> Response {
    Response::from_string("")
        .with_status_code(303)
        .with_header(Header::from_bytes(&b"Location"[..], location.as_ref()).unwrap())
}

/// Render the standard header that is the same across all pages.
fn view_html_head(page_title: &str) -> Markup {
    html! {
        (DOCTYPE)
        head {
            meta charset="utf-8";
            link rel="preconnect" href="https://fonts.googleapis.com";
            link rel="preconnect" href="https://fonts.gstatic.com" crossorigin;
            link href="https://fonts.googleapis.com/css2?family=Work+Sans:ital,wght@0,700..800;1,900&family=Atkinson+Hyperlegible:ital,wght@0,400;0,700;1,400&display=swap" rel="stylesheet";
            meta name="viewport" content="width=device-width, initial-scale=1";
            title { (page_title) }
            style { (get_stylesheet()) }
        }
    }
}

// In debug mode, we load the stylesheet from disk on the fly, so you can edit
// without having to rebuild the server.
#[cfg(debug_assertions)]
fn get_stylesheet() -> Markup {
    let data = std::fs::read_to_string("src/style.css")
        .expect("Need to run from repo root in debug mode.");
    html! { (data) }
}

// For a release build, we embed the stylesheet into the binary.
#[cfg(not(debug_assertions))]
fn get_stylesheet() -> Markup {
    let data = include_str!("style.css");
    html! { (data) }
}

fn view_header(ctx: &Context) -> Markup {
    html! {
        nav {
            h1 {
                a href=(ctx.config.server.prefix) { "Predict-o-matic" }
            }
            " "
            span .balance {
                (format!("$\u{200a}{:.2}", ctx.user_points))
            }
            " "
            span .user {
                (ctx.user.email)
            }
        }
    }
}

pub fn handle_get(tx: &mut db::Transaction, ctx: &Context, path: &[&str]) -> db::Result<Response> {
    match path {
        [] | [""] => index::handle_index(tx, ctx),
        ["market", market_slug] => market::handle_market(tx, ctx, market_slug),
        _ => Ok(not_found("Not found.")),
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
        _ => Ok(not_found("Not found.")),
    }
}
