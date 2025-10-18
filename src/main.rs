// Predict-o-matic -- A webapp for facilitating internal prediction markets
// Copyright 2025 Chorus One

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

// Apapted from Hack-o-matic <https://github.com/ChorusOne/hackomatic>.
// Copyright 2024 Chorus One, licensed Apache 2.0.

use std::io::Cursor;
use std::str::FromStr;
use std::time::Instant;

use tiny_http::{HeaderField, Method, Request, Server};

use crate::config::{AppConfig, Config, DatabaseConfig, MarketConfig, ServerConfig};
use crate::database as db;
use crate::routes::{internal_error, not_found, service_unavailable};

mod config;
mod database;
mod model;
mod routes;

type Response = tiny_http::Response<Cursor<Vec<u8>>>;

fn load_config() -> Config {
    let mut args = std::env::args();

    // Skip the program name.
    args.next();

    let config_fname = match args.next() {
        Some(fname) => fname,
        None => panic!("Expected config file path as first argument."),
    };

    let config_toml = match std::fs::read_to_string(&config_fname) {
        Ok(string) => string,
        Err(err) => panic!("Failed to read {config_fname:?}: {err:?}"),
    };

    match toml::from_str(&config_toml) {
        Ok(config) => config,
        Err(err) => panic!("Failed to parse {config_fname:?}: {err:?}"),
    }
}

fn connect_database<'conn>(
    raw_connection: &'conn sqlite::Connection,
) -> db::Result<db::Connection<'conn>> {
    // Change the database to WAL mode if it wasn't already. Set the busy
    // timeout to 30 milliseconds, so readers and writers can wait for each
    // other a little bit.
    raw_connection.execute("PRAGMA locking_mode = NORMAL;")?;
    raw_connection.execute("PRAGMA busy_timeout = 30;")?;
    raw_connection.execute("PRAGMA journal_mode = WAL;")?;
    raw_connection.execute("PRAGMA foreign_keys = TRUE;")?;
    Ok(db::Connection::new(raw_connection))
}

pub struct User {
    email: String,
    is_admin: bool,
}

fn handle_request(
    config_app: &AppConfig,
    config_server: &ServerConfig,
    connection: &mut db::Connection,
    request: &mut Request,
    log_line: &mut String,
) -> db::Result<Response> {
    // In development, we can have a fallback email for when the X-Email header
    // is not present. In release builds we disable that mechanism, because it
    // sidesteps authentication and is therefore unsafe.
    #[cfg(debug_assertions)]
    let mut email = config_server.unsafe_user_email.clone();

    #[cfg(not(debug_assertions))]
    let mut email = None;

    let header_x_email = HeaderField::from_str("X-Email").unwrap();
    for header in request.headers() {
        if header.field == header_x_email {
            // We need to clone the value, because later on we might need to
            // read the request body, and we can't do that with a reference to
            // a header.
            email = Some(header.value.to_string());
        }
    }

    let email = match email {
        None => {
            return Ok(Response::from_string("Missing authentication header.").with_status_code(401));
        }
        Some(email) => email,
    };

    // In the database, owner columns rely on the fact that SYSTEM is a reserved name.
    assert_ne!(
        email, "SYSTEM",
        "SYSTEM is a reserved name, it cannot be used by users."
    );

    *log_line = format!("{:4?} {} {}", request.method(), request.url(), email);

    let user = User {
        is_admin: email == config_app.admin_email,
        email,
    };

    let url_clone = match request.url().strip_prefix(&config_server.prefix) {
        Some(url) => url.to_string(),
        None => {
            return Ok(not_found(format!(
                "Not found, try {}",
                config_server.prefix
            )));
        }
    };
    let path_segments: Vec<_> = url_clone.trim_start_matches('/').split('/').collect();

    // For post requests, read the body. We need to do this once. The handler
    // may be retried, but the body we can only consume once.
    let mut body = String::new();
    if request.method() == &Method::Post {
        // Read the body, ignore any IO errors there. In most cases this is
        // probably fine and we'll fail elsewhere, but it might happen that
        // we read a truncated body and fail half-way.
        if request.as_reader().read_to_string(&mut body).is_err() {
            return Ok(internal_error("Failed to read full request body."));
        }
    }

    with_transaction(connection, |tx| {
        let ctx = routes::Context::new(config_app, config_server, &user, tx)?;

        match request.method() {
            Method::Post => routes::handle_post(tx, &ctx, &path_segments, &body),
            // Assume everything else is a GET request.
            _ => routes::handle_get(tx, &ctx, &path_segments),
        }
    })
}

/// Run `f` in a transaction, retrying a few times if the database is busy.
///
/// SQLite does not support concurrent writes, but we do spawn multiple server
/// threads. It might happen that one of them encounters a concurrency error and
/// needs to restart the transaction, try that a few times before finally gving up.
fn with_transaction<F>(connection: &mut db::Connection, mut f: F) -> db::Result<Response>
where
    F: FnMut(&mut db::Transaction) -> db::Result<Response>,
{
    for attempt in 0.. {
        let mut tx = connection.begin()?;
        match f(&mut tx) {
            Ok(response) => {
                // Commit on success responses (we assume redirects to be success
                // as well, for example for use after submitting a form). If we
                // encounter any error, roll back. We do this here because
                // handlers cannot call `tx.rollback()`, because it consumes the
                // transaction.
                if response.status_code().0 < 400 {
                    tx.commit()?;
                } else {
                    tx.rollback()?;
                }
                return Ok(response);
            }
            Err(err) if err.code == Some(5) => {
                tx.rollback()?;
                println!("Database is locked (attempt {}): {err:?}", attempt + 1);
                // The database is locked by a writer. Retry if we haven't
                // retried too many times already.
                if attempt + 1 < 6 {
                    continue;
                } else {
                    return Ok(service_unavailable(
                        "The database is busy, wait a few seconds and try again.",
                    ));
                }
            }
            Err(err) => {
                // Try to roll back, but if it doesn't work, we are going to
                // open a new connection anyway.
                let _ = tx.rollback();
                return Err(err);
            }
        }
    }
    unreachable!("The number of continuations is bounded.");
}

fn serve_until_error(
    config_app: &AppConfig,
    config_server: &ServerConfig,
    connection: &mut db::Connection,
    server: &Server,
) {
    loop {
        let mut fatal_error = None;
        let mut request = server.recv().unwrap();
        let start_time = Instant::now();

        let mut log_line = "Unparsed request".to_string();
        let response = match handle_request(
            config_app,
            config_server,
            connection,
            &mut request,
            &mut log_line,
        ) {
            Ok(resp) => {
                println!(
                    "{log_line} -> {} [{:.3} ms]",
                    resp.status_code().0,
                    (start_time.elapsed().as_micros() as f32) * 1e-3
                );
                resp
            }
            Err(err) => {
                // Some unrecoverable error happened.
                println!("{log_line} -> Error: {err:?}");
                fatal_error = Some(err);
                internal_error("Internal server error.")
            }
        };

        if let Err(err) = request.respond(response) {
            println!("Error writing response: {err:?}");
        }
        if let Some(err) = fatal_error {
            println!("Restarting server loop due to error: {err:?}");
            return;
        }
    }
}

fn run_server(
    config_app: &AppConfig,
    config_server: &ServerConfig,
    config_database: &DatabaseConfig,
) {
    // For now, don't bother making the server multithreaded. See the comment
    // in Hackomatic for more details, something something SQLite concurrent
    // writers ...
    let server = Server::http(&config_server.listen).unwrap();

    #[cfg(debug_assertions)]
    let user_annotation = match &config_server.unsafe_user_email {
        None => "".to_string(),
        Some(email) => format!(" (development user {email})"),
    };

    #[cfg(not(debug_assertions))]
    let user_annotation = "";

    println!(
        "Serving on http://{}{}{} ...",
        config_server.listen, config_server.prefix, user_annotation,
    );

    loop {
        let raw_connection = sqlite::open(&config_database.path).expect("Failed to open database");
        let mut connection =
            connect_database(&raw_connection).expect("Failed to initialize database.");

        // Handle requests until we encounter a database error.
        // At that point we loop and open a fresh connection.
        serve_until_error(config_app, config_server, &mut connection, &server);
    }
}

/// Ensure the schema exists, and populate initial markets from the config file.
fn initialize_database(config: &DatabaseConfig, markets: &[MarketConfig]) -> db::Result<()> {
    let raw_connection = sqlite::open(&config.path).expect("Failed to open database.");
    let mut connection = connect_database(&raw_connection).expect("Failed to connect to database.");
    let mut tx = connection.begin()?;
    db::ensure_schema_exists(&mut tx)?;
    model::ensure_markets(&mut tx, markets)?;
    tx.commit()
}

fn main() {
    use std::sync::Arc;
    use std::thread;

    let config = load_config();

    // Ensure the schema exists, and populate the markets from the config
    // file if applicable.
    initialize_database(&config.database, &config.markets).expect("Failed to initialize database.");

    let config_app = Arc::new(config.app);
    let config_database = Arc::new(config.database);

    let mut threads = Vec::new();

    let config_app_main = config_app.clone();
    let config_database_main = config_database.clone();
    let thread_main =
        thread::spawn(move || run_server(&config_app_main, &config.server, &config_database_main));
    threads.push(thread_main);

    #[cfg(debug_assertions)]
    for config_server in config.demo_servers {
        let config_app_demo = config_app.clone();
        let config_database_demo = config_database.clone();
        threads.push(thread::spawn(move || {
            run_server(&config_app_demo, &config_server, &config_database_demo)
        }));
    }

    for thread in threads.into_iter() {
        thread.join().unwrap();
    }
}
