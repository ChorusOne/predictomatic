# Predict-o-matic

The Predict-o-matic is a simple webapp for running internal prediction markets.
It handles market creation, trading, resolution, and it displays the market's
current predictions. It needs an external system such as [OAuth2 Proxy][o2proxy]
for user management and authentication.

## Building

The Predict-o-matic is written in Rust and builds with Cargo:

    cargo build --release
    target/release/predictomatic predictomatic.toml

## Deploying

The binary starts a webserver that listens on the [configured](#configuration)
port. This webserver is expected to be protected by a reverse-proxy that sets
the `X-Email` header. The reverse proxy should handle authentication and
authorization. This is a convenient way to ensure that all people in your
organization can join and predict without having to create an account anywhere.

One possible setup is to use Nginx and [OAuth2 Proxy][o2proxy]. To make OAuth2
Proxy pass the user’s email address, enable the `--set-xauthrequest` option.
The documentation [contains an example][o2-nginx] for how to configure Nginx to
set the `X-Email` header when using `auth_request`.

The Predict-o-matic stores all data in a SQLite database. To back up the
database, one convenient way is to use [`VACUUM INTO`][vacuum]:

    $ sqlite3 predictomatic.sqlite3
    sqlite> VACUUM INTO 'predictomatic-backup.sqlite3';

[o2proxy]:  https://oauth2-proxy.github.io/oauth2-proxy/
[o2-nginx]: https://oauth2-proxy.github.io/oauth2-proxy/configuration/overview#configuring-for-use-with-the-nginx-auth_request-directive
[vacuum]:   https://sqlite.org/lang_vacuum.html

## Configuration

There is a single toml configuration file. See `predictomatic.toml` for an
example. See `src/config.rs` for documentation of the fields.

For local testing where no reverse proxy to set the `X-Email` header is
available, you can set `debug.unsafe_default_email` to an email address that
will be used when no `X-Email` header is present. This feature is of course
unsafe to use in production.

## Development

For local development, it can be helpful to set up multiple configuration files
configured with different port and debug user, but otherwise sharing the same
settings, in particular sharing the database. You can even set up auto-reload
with e.g. [Entr], [Hivemind], and a `Procfile` like this:

```Procfile
user_a: git ls-files | entr -r cargo run -- predictomatic_user_a.toml
user_b: git ls-files | entr -r cargo run -- predictomatic_user_b.toml
```

[Entr]:     https://eradman.com/entrproject/
[Hivemind]: https://github.com/DarthSim/hivemind

## License

The Predict-o-matic is a fork of the [Hack-o-matic][hackomatic] by Chorus One.
Both are licensed under the Apache 2.0 License. A copy of the license is
included in the root of the repository.

[hackomatic]: https://github.com/ChorusOne/hackomatic
