// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

// Apapted from Hack-o-matic <https://github.com/ChorusOne/hackomatic>.
// Copyright 2024 Chorus One, licensed Apache 2.0.

use maud::{DOCTYPE, Markup, html};
use tiny_http::Header;

use crate::config::Config;
use crate::database as db;
use crate::model::{self, Amount, AssetId, Market};
use crate::{Response, User};

fn respond_html(markup: Markup) -> Response {
    Response::from_string(markup.into_string()).with_header(
        Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap(),
    )
}

fn respond_error<R: Into<String>>(reason: R) -> Response {
    let page = html! {
        (view_html_head("Predict-o-matic Error"))
        body {
            div class="main-error" {
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

// Same for the script.
#[cfg(debug_assertions)]
fn get_predict_script() -> Markup {
    let data = std::fs::read_to_string("src/predict.js")
        .expect("Need to run from repo root in debug mode.");
    maud::PreEscaped(data)
}

#[cfg(not(debug_assertions))]
fn get_predict_script() -> Markup {
    maud::PreEscaped(include_str!("predict.js").to_string())
}

fn view_email<'a>(config: &Config, email: &'a str) -> &'a str {
    match email.strip_suffix(&config.app.email_suffix) {
        Some(stripped) => stripped,
        None => email,
    }
}

struct Context<'a> {
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
}

fn view_header(ctx: &Context) -> Markup {
    html! {
        nav {
            h1 {
                a href=(ctx.config.server.prefix) { "Predict-o-matic" }
            }
            " "
            span class="balance" {
                (format!("$\u{200a}{:.2}", ctx.user_points))
            }
            " "
            span class="user" {
                (ctx.user.email)
            }
        }
    }
}

fn view_index(ctx: &Context, markets: &[Market]) -> Markup {
    html! {
        (view_html_head("Predict-o-matic"))
        body {
            (view_header(ctx))
            div class="main" {
                section {
                    h1 { "Markets" }
                    @for market in markets {
                        div class="market" {
                            h2 {
                                a href=(ctx.market_url(market, "")) {
                                    (market.title)
                                }
                            }
                            p {
                                "TODO: Add a summary of the prediction here."
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn handle_index(
    config: &Config,
    tx: &mut db::Transaction,
    user: &User,
) -> db::Result<Response> {
    let ctx = Context::new(config, user, tx)?;

    // TODO: iterate the markets at once rather than getting them by id to save
    // a bit of interop, but it's SQLite so we are not even saving a round-trip,
    // and it's a hackathon so YOLO.
    let market_slugs: Vec<_> = db::get_market_slugs(tx)?.collect();
    let mut markets = Vec::new();
    for res_slug in market_slugs {
        let slug = res_slug?;
        markets.push(model::get_market_by_slug(tx, &slug)?.expect("We know the market exists."));
    }

    let body = view_index(&ctx, &markets);
    Ok(respond_html(body))
}

fn view_market(ctx: &Context, market: &Market) -> Markup {
    let default_deposit = AssetId::POINTS.micros(10_000_000).min(ctx.user_points);
    let total_deposited = market.total_deposited();
    let ps = market.implied_distribution().ps();
    let p_yes = ps[0];
    let p_no = ps[1];

    html! {
        (view_html_head("Predict-o-matic"))
        body {
            (view_header(ctx))
            div class="main" {
                section {
                    h1 { (market.title) }
                    p { "I need to summarize the prediction here." }
                    h2 { "Resolution criteria" }
                    p { (market.description) }
                    h2 { "Participants" }
                    p { "I could show a table here of participants, ranked by volume and PnL." }
                    h2 { "Activity" }
                    p { "I could show a log of trades and comments here." }
                }
                aside {
                    table {
                        tr {
                            td { "Liquidity" }
                            td class="num" { (format!("$\u{200a}{total_deposited:.2}")) }
                        }
                        tr {
                            td { "Yes" }
                            td class="num" { (format!("$\u{200a}{p_yes:.2}")) }
                        }
                        tr {
                            td { "No" }
                            td class="num" { (format!("$\u{200a}{p_no:.2}")) }
                        }
                    }

                    h3 { "Your balance" }
                    table {
                        tr {
                            td { "Yes" }
                            td class="num" { "0.00" }
                        }
                        tr {
                            td { "No" }
                            td class="num" { "0.00" }
                        }
                        tr {
                            td { "Market value" }
                            td class="num" { "$\u{200a}0.00" }
                        }
                        tr {
                            td { "Deposited" }
                            td class="num" { "$\u{200a}0.00" }
                        }
                        tr {
                            td { "Unrealized PnL" }
                            td class="num" { "$\u{200a}0.00" }
                        }
                    }
                    h3 { "Deposit" }
                    form method="post" action=(ctx.market_url(market, "/deposit")) {
                        label {
                            "Amount "
                            input
                                name="amount"
                                type="number"
                                min="0.00"
                                max=(ctx.user_points)
                                step="any"
                                value=(format!("{default_deposit:.2}"));
                        }
                        button type="submit" { "Deposit" }
                    }
                }
            }
        }
    }
}

pub fn handle_market(
    config: &Config,
    tx: &mut db::Transaction,
    user: &User,
    market_slug: &str,
) -> db::Result<Response> {
    let ctx = Context::new(config, user, tx)?;
    let market = match model::get_market_by_slug(tx, market_slug)? {
        None => return Ok(not_found("No such market exists.")),
        Some(market) => market,
    };

    for (o, bs) in market.balances.iter() {
        println!("{o} -> {bs:?}");
    }

    let body = view_market(&ctx, &market);
    Ok(respond_html(body))
}

pub fn handle_deposit(
    config: &Config,
    tx: &mut db::Transaction,
    user: &User,
    market_slug: &str,
    body: &str,
) -> db::Result<Response> {
    let ctx = Context::new(config, user, tx)?;
    let market = match model::get_market_by_slug(tx, market_slug)? {
        None => return Ok(not_found("No such market exists.")),
        Some(market) => market,
    };

    let mut amount = AssetId::POINTS.zero();

    for (key, value) in form_urlencoded::parse(body.as_bytes()) {
        match key.as_ref() {
            "amount" => match AssetId::POINTS.parse_amount(value.as_ref()) {
                None => return Ok(bad_request("Failed to parse amount.")),
                Some(n) => amount = n,
            },
            _ => return Ok(bad_request("Unexpected form data.")),
        }
    }

    if amount <= AssetId::POINTS.zero() {
        return Ok(bad_request("Amount must be greater than 0."));
    }

    model::create_deposit(tx, &market, amount, &user.email)?;

    Ok(redirect_see_other(ctx.market_url(&market, "")))
}

/// Validate user inputs against length limits and Unicode subset.
///
/// Users should be able to input text, but allowing any Unicode code point
/// creates a can of worms where you can use distracting emoji, or reverse the
/// text direction for all following content, use the mathematical symbols to do
/// "markup", etc. So ban most of Unicode, but allow more than just ASCII
/// because Tomás and Mikołaj are valid non-ASCII names. This is very crude but
/// it'll do.
///
/// Returns a description of the violaton on error.
fn validate_string(label: &'static str, max_len: usize, input: &str) -> Result<(), String> {
    if input.is_empty() {
        return Err(format!("{label} must not be empty."));
    }

    if input.len() > max_len {
        return Err(format!("{label} may not be longer than {max_len} bytes."));
    }

    for ch in input.chars() {
        // Control characters are not allowed (including newline).
        // Space (U+0020) is the first one that is allowed.
        if ch < '\u{20}' {
            return Err(format!(
                "{label} may not contain control characters (including newlines)."
            ));
        }

        // Allow General Punctuation (U+2000 through U+206F).
        if ch >= '\u{2000}' && ch < '\u{2070}' {
            continue;
        }

        // Allow Basic Latin, the supplement, extended Latin, modifiers,
        // diacritics, then a few other languages like Greek and Cyrillic, but
        // stop after Arabic.
        if ch >= '\u{0780}' {
            return Err(format!(
                "{label} contains an invalid character: ‘{ch}’ (U+{:04X}) is not allowed.",
                ch as u32
            ));
        }
    }

    Ok(())
}
