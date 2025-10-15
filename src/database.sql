-- Predict-o-matic -- A webapp for facilitating internal prediction markets
-- Copyright 2025 Chorus One

-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- A copy of the License has been included in the root of the repository.

-- Apapted from Hack-o-matic <https://github.com/ChorusOne/hackomatic>.
-- Copyright 2024 Chorus One, licensed Apache 2.0.

-- To be used with https://github.com/ruuda/squiller v0.5.0

-- @begin ensure_schema_exists()
create table if not exists accounts
( id       integer primary key
  -- 0 for the native asset (points), or the id of an outcome for
  -- market accounts.
, asset_id integer not null
  -- Email address for user-owned accounts, or "SYSTEM" for system-owned
  -- accounts.
, owner    string not null
  -- Credits minus debits.
, balance  integer not null
  -- Every user can have at most one account per asset.
, unique (asset_id, owner)
);

-- Events group one or more transfers. We could also call them "transaction",
-- but that's confusing in a database.
create table if not exists events
( id          integer primary key
, created_at  string not null
, created_by  string not null
, description string not null
);

create table if not exists transfers
( id              integer primary key
, event_id        integer not null references events (id)
, from_account_id integer not null references accounts (id)
, to_account_id   integer not null references accounts (id)
, amount          integer not null
);

create table if not exists markets
( id          integer primary key
, created_at  string  not null
, title       string  not null
);

create table if not exists outcomes
( id        integer primary key
, market_id integer not null references markets (id)
, value     string not null
);

-- @end ensure_schema_exists()
