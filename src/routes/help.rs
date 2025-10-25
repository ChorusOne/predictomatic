// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use maud::{Markup, html};

use crate::Response;
use crate::routes::{Context, Result, index, respond_html, view_header, view_html_head};

struct HelpSection {
    title: &'static str,
    anchor: String,
    body: Vec<Markup>,
}

/// Return a "parsed" version of `docs/user_guide.md`.
///
/// This interpolates the following variables:
/// * `$POINTS -> points`.
fn get_help_sections(points: &str) -> Vec<HelpSection> {
    let raw_md = include_str!("../../docs/user_guide.md");

    let push_paragraph = |body: &mut Vec<_>, paragraph: &mut String| {
        if !paragraph.is_empty() {
            let mut tmp = String::new();
            std::mem::swap(&mut tmp, paragraph);
            if tmp.contains("$POINTS") {
                tmp = tmp.replace("$POINTS", points);
            }
            // TODO: Also insert no-break hair spaces before dollar signs?
            body.push(maud::PreEscaped(tmp));
        }
    };

    let push_section = |result: &mut Vec<_>, title: &'static str, body: &mut Vec<_>| {
        let mut tmp_body = Vec::new();
        std::mem::swap(&mut tmp_body, body);

        let section = HelpSection {
            title,
            anchor: title.to_lowercase().replace(" ", "-"),
            body: tmp_body,
        };
        result.push(section);
    };

    let mut result = Vec::new();
    let mut paragraph = String::new();
    let mut body = Vec::new();
    let mut title = "";
    let mut lines = raw_md.lines();

    // Note, the loop below on purpose loses all paragraphs before the first heading.

    loop {
        match lines.next() {
            Some(line) if line.starts_with("# ") => {
                // We skip over the document title, we have our own <h1> below.
            }
            Some(line) if line.starts_with("## ") => {
                // We encountered a new section. Push the preceding one.
                push_paragraph(&mut body, &mut paragraph);
                push_section(&mut result, title, &mut body);
                title = &line[3..];
            }
            Some(line) if line.is_empty() => {
                // A blank line signals the start of a new paragraph.
                push_paragraph(&mut body, &mut paragraph);
            }
            Some(line) => {
                // If it's not a heading, we assume it's a body line.
                paragraph.push('\n');
                paragraph.push_str(line);
            }
            None => {
                // Flush the final paragraph and section at EOF.
                push_paragraph(&mut body, &mut paragraph);
                push_section(&mut result, title, &mut body);
                return result;
            }
        }
    }
}

fn view_help(ctx: &Context) -> Markup {
    let user_points = format!("{:.2}", ctx.user_points);
    html! {
        (view_html_head(ctx.prefix, "Predict-o-matic"))
        body {
            (view_header(ctx))
            div .main .wider {
                section {
                    h1 { "User manual" }

                    @for section in get_help_sections(&user_points) {
                        @if !section.title.is_empty() {
                            h2 id=(section.anchor) {
                                a href={"#" (section.anchor)} { (section.title) }
                            }
                        }
                        @for paragraph in section.body {
                            p { (paragraph) }
                        }
                    }
                }
                aside { (index::view_main_aside(ctx)) }
            }
        }
    }
}

pub fn handle_help(ctx: &Context) -> Result<Response> {
    Ok(respond_html(view_help(ctx)))
}
