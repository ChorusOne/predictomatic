// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

// Apapted from Hack-o-matic <https://github.com/ChorusOne/hackomatic>.
// Copyright 2024 Chorus One, licensed Apache 2.0.

use serde::{self, Deserialize};

/// Application configuration.
///
/// The configuration is trivial, but split into structs anyway to make the
/// structure of the corresponding toml file a bit nicer.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub app: AppConfig,
    #[serde(default)]
    pub debug: DebugConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    #[serde(default, rename = "market")]
    pub markets: Vec<MarketConfig>,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    /// The email address of the user who can administrate markets.
    pub admin_email: String,

    /// The suffix to remove from user emails when listing them.
    pub email_suffix: String,

    /// The opening balance of new users, in 10^-6 points.
    pub opening_balance_micropoints: i64,
}

#[derive(Debug, Default, Deserialize)]
pub struct DebugConfig {
    /// Use this as fallback email when the `X-Email` header is not set.
    ///
    /// In a production deployment, `X-Email` should be set by an authenticating
    /// proxy such as Oauth2-Proxy. For local development, we allow the header
    /// to be omitted and instead assume this email when no header is present.
    pub unsafe_default_email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    /// The interface address and port to listen on, e.g. `127.0.0.1:5591`.
    pub listen: String,

    /// The url prefix, in case the app is not hosted at the root of a domain.
    ///
    /// E.g. `/predict-o-matic`.
    pub prefix: String,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    /// Path to the database file.
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub enum MarketKind {
    #[serde(rename = "binary")]
    Binary,
    #[serde(rename = "date")]
    Date,
}

#[derive(Debug, Deserialize)]
pub struct MarketConfig {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub kind: MarketKind,
    pub outcomes: Vec<String>,
}
