// Predict-o-matic -- A webapp for facilitating internal prediction markets
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
pub struct AppConfig {
    /// The email address of the user who can administer markets.
    pub admin_email: String,

    /// The suffix to remove from user emails when displaying them.
    pub email_suffix: String,

    /// The opening balance of new users, in 10^-6 points.
    pub opening_balance_micros: i64,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    /// The interface address and port to listen on, e.g. `127.0.0.1:5591`.
    pub listen: String,

    /// The url prefix, in case the app is not hosted at the root of a domain.
    ///
    /// E.g. `/predict-o-matic`.
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
}
