// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use crate::model::{Amount, AssetId};
use crate::routes::{Context, Result};

/// Parse the key `amount` from a form body, into a positive amount of points.
pub fn parse_form_amount(ctx: &Context, body: &str) -> Result<Amount> {
    let mut amount = AssetId::POINTS.zero();

    for (key, value) in form_urlencoded::parse(body.as_bytes()) {
        match key.as_ref() {
            "amount" => {
                // A dollar sign is optional, we include it in the UI by default
                // to make things clearer, but it's actually annoying to type so
                // it's not mandatory.
                match AssetId::POINTS.parse_amount(value.as_ref().trim_start_matches('$')) {
                    None => return ctx.bad_request("Failed to parse amount."),
                    Some(n) => amount = n,
                }
            }
            _ => return ctx.bad_request("Unexpected form data."),
        }
    }

    if amount <= AssetId::POINTS.zero() {
        return ctx.bad_request(format!("Amount must be greater than 0, but got {amount}."));
    }

    Ok(amount)
}
