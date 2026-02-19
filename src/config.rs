// Predictomatic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

// Apapted from Hack-o-matic <https://github.com/ChorusOne/hackomatic>.
// Copyright 2024 Chorus One, licensed Apache 2.0.

use std::fmt;

use serde::{self, Deserialize, Serialize};

/// Application configuration.
///
/// See also the example `predictomatic.toml` in the repository root.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub app: AppConfig,

    pub database: DatabaseConfig,

    /// Configuration for the "production" http server.
    pub server: ServerConfig,

    /// Additional servers to spawn for local development.
    #[serde(default, rename = "demo_server")]
    pub demo_servers: Vec<ServerConfig>,

    #[serde(default, rename = "market")]
    pub markets: Vec<MarketConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    /// The email address of the user who can administer markets.
    pub admin_email: String,

    /// The suffix to remove from user emails when displaying them.
    pub email_suffix: String,

    /// The opening balance of new users, in 10^-6 points.
    pub opening_balance_micros: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    /// The interface address and port to listen on, e.g. `127.0.0.1:5591`.
    pub listen: String,

    /// The url prefix, in case the app is not hosted at the root of a domain.
    ///
    /// E.g. `/predictomatic`. If the prefix is not empty, it must start with
    /// a slash. The prefix must not end with a slash.
    pub prefix: String,

    /// Use this as fallback email when the `X-Email` header is not set.
    ///
    /// In a production deployment, `X-Email` should be set by an authenticating
    /// proxy such as Oauth2-Proxy. For local development, this is a pain to
    /// configure, so instead we can configure additional demo servers where for
    /// any request handled by that server, we assume the user with this given
    /// email address is logged in.
    ///
    /// For safety, this field is only supported in development builds.
    /// A release build will refuse to load unsafe configurations.
    #[cfg(debug_assertions)]
    pub unsafe_user_email: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseConfig {
    /// Path to the database file.
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum MarketKind {
    #[serde(rename = "binary")]
    Binary,
    #[serde(rename = "date")]
    Date,
}

impl fmt::Display for MarketKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.serialize(f)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketConfig {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub kind: MarketKind,
    pub outcomes: Vec<String>,

    /// The amount to bootstrap the AMM with, in 10^-6 points.
    ///
    /// This is paid for by minting new points from the system account.
    pub fund_micros: i64,

    /// Date at which the market opens.
    ///
    /// Should be TOML datetime with Z offset, e.g. `2026-02-19T14:12:00Z`.
    /// This can be used to create markets that open in the future. When not
    /// specified, the market opens immediately at creation time.
    pub opens_at: Option<toml::value::Datetime>,

    /// Date at which the market closes.
    ///
    /// Should be a TOML datetime with Z offset, like `opens`. This can be used
    /// to set a future deadline by which the market closes. This is useful for
    /// preventing trades in a market for which the resolution is known, but
    /// which is not resolved yet. For example, when the market is about
    /// something that will become known on a given date in the weekend, but the
    /// admin can only resolve the market next working day.
    ///
    /// When not specified, the market will not have a specific close date.
    pub closes_at: Option<toml::value::Datetime>,
}

/// Format a TOML datetime as ISO-8601 UTC time that we support in the database.
fn as_iso8601(dt: &toml::value::Datetime) -> String {
    assert_eq!(
        dt.offset,
        Some(toml::value::Offset::Z),
        "Unsupported datetime {dt}, expected `Z` offset suffix.",
    );
    assert!(
        dt.date.is_some(),
        "Unsupported datetime {dt}, expected a date part."
    );
    assert!(
        dt.time.is_some(),
        "Unsupported datetime {dt}, expected a time part."
    );
    dt.to_string()
}

impl MarketConfig {
    pub fn opens_at_iso8601(&self) -> Option<String> {
        self.opens_at.as_ref().map(as_iso8601)
    }

    pub fn closes_at_iso8601(&self) -> Option<String> {
        self.closes_at.as_ref().map(as_iso8601)
    }
}

#[cfg(test)]
mod test {
    use super::Config;

    #[test]
    fn example_config_can_be_parsed() {
        let example_toml = std::fs::read_to_string("predictomatic.toml")
            .expect("Example config in repo should be readable.");
        let _: Config = toml::from_str(&example_toml).expect("Example config should be parseable.");
    }
}
