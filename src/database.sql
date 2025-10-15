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
, check (amount > 0)
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

-- Create the system points account.
insert into accounts (id, asset_id, owner, balance)
  values (0, 0, 'SYSTEM', 0)
  on conflict do nothing;

-- @end ensure_schema_exists()

-- Return the account id a given (owner, asset_id) pair.
-- @query get_account_id(owner: str, asset_id: i64) ->? i64
select id from accounts where owner = :owner and asset_id = :asset_id;

-- Create a new account for a given (owner, asset_id) pair, return its id.
-- @query create_account(owner: str, asset_id: i64) ->1 i64
insert into accounts (owner, asset_id, balance)
  values (:owner, :asset_id, 0)
  returning id;

-- Start a new event that can have transfers attached to it, returns its id.
-- @query create_event(created_by: str, description: str) ->1 i64
insert into events (created_by, description)
  values (:created_by, :description)
  returning id;

-- Record a transfer between two accounts.
-- @begin create_transfer(
--   event_id: i64,
--   from_account_id: i64,
--   to_account_id: i64,
--   amount: i64,
-- )
insert into transfers (event_id, from_account_id, to_account_id, amount)
  values (:event_id, :from_account_id, :to_account_id, :amount);

update accounts
  set   balance = balance - amount
  where account_id = from_account_id;

update accounts
  set   balance = balance + amount
  where account_id = to_account_id;

-- @end create_transfer
