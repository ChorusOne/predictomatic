// Predictomatic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use crate::model::{Amount, AssetId};
use crate::routes::{Context, Result};

use maud::{Markup, html};

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

/// View the date part of an ISO-8601 string, with full time as tooltip.
pub fn view_date(iso8601: &str) -> Markup {
    assert_eq!(
        iso8601.len(),
        20,
        "Input must be of form 'YYYY-MM-DDTHH:mm:ssZ'."
    );
    assert_eq!(&iso8601[10..11], "T");
    assert_eq!(&iso8601[19..20], "Z");
    let date = &iso8601[..10];
    let time = &iso8601[11..19];
    // Add the full timestamp in the tooltip, in a ISO-8601 like
    // format that is actually readable to humans.
    html! { span .num title={(date) " " (time) " UTC"} { (date) } }
}
