-- Predict-o-matic -- A webapp for facilitating internal prediction markets
-- Copyright 2025 Chorus One

-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- A copy of the License has been included in the root of the repository.

-- Apapted from Hack-o-matic <https://github.com/ChorusOne/hackomatic>.
-- Copyright 2024 Chorus One, licensed Apache 2.0.

-- To be used with <https://github.com/ruuda/squiller>, you need commit
-- c83d64f644805e70 or later.

-- @begin ensure_schema_exists()
create table if not exists accounts
( id       integer primary key
  -- 0 for the global accounts (points owned by the user or system), or the id
  -- of the market for accounts associated with a market.
, market_id integer not null
  -- 0 for the native asset (points), or the id of an outcome for
  -- market accounts.
, asset_id integer not null
  -- Email address for user-owned accounts, or "SYSTEM" for system-owned
  -- accounts.
, owner    text not null
  -- Credits minus debits.
, balance  integer not null
  -- The minimum allowed balance.
, min_balance integer null
  -- The maximum allowed balance.
, max_balance integer null
  -- Every user can have at most one account per asset per market.
, unique (market_id, asset_id, owner)
  -- Balance constraints must be satisfied.
, check (balance >= min_balance)
, check (balance <= max_balance)
);

-- Events group one or more transfers. We could also call them "transaction",
-- but that's confusing in a database.
create table if not exists events
( id          integer primary key
, created_at  text not null
, created_by  text not null
, description text not null
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
, slug        text not null
, created_at  text not null
, kind        text not null
, title       text not null
, description text not null
, resolved_in integer null references events (id)
, unique (slug)
, unique (title)
);

create table if not exists outcomes
( id        integer primary key
, market_id integer not null references markets (id)
, value     text not null
, unique (market_id, value)
);

-- For resolved markets, we track the realized profit per participant.
-- Every profit links to a resolution event.
create table if not exists realized_profits
( id               integer primary key
, market_id        integer not null references markets (id)
, event_id         integer not null references events (id)
, owner            text    not null
, amount_in        integer not null
, amount_out       integer not null
, unique (market_id, owner)
, check (amount_in > 0)
);

-- Create the system points account. This balance can only go negative, as we
-- mint points from here.
insert into
  accounts
    (id, market_id, asset_id, owner, balance, min_balance, max_balance)
  values
    (0, 0, 0, 'SYSTEM', 0, null, 0)
  on conflict do nothing;

-- @end ensure_schema_exists()

-- Return the account id a given (market_id, asset_id, owner).
-- @query get_account_id(market_id: i64, asset_id: i64, owner: str) ->? i64
select id from accounts
  where true
    and (market_id = :market_id)
    and (asset_id = :asset_id)
    and (owner = :owner);

-- Create a new account for a given (market_id, asset_id, owner), return its id.
-- @query create_account(
--   market_id: i64,
--   asset_id: i64,
--   owner: str,
--   min_balance: i64?,
--   max_balance: i64?,
-- ) ->1 i64
insert into
  accounts
    (market_id, asset_id, owner, balance, min_balance, max_balance)
  values
    (:market_id, :asset_id, :owner, 0, :min_balance, :max_balance)
  returning id;

-- Return the balance for the given account.
-- @query get_account_balance(account_id: i64) ->1 i64
select balance from accounts where id = :account_id;

-- Start a new event that can have transfers attached to it, returns its id.
-- @query create_event(created_by: str, description: str) ->1 i64
insert into
  events
  ( created_at
  , created_by
  , description
  )
  values
  ( strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
  , :created_by
  , :description
  )
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
  set   balance = balance - :amount
  where (id = :from_account_id)
    and (:to_account_id is not null) and (:amount > 0);

update accounts
  set   balance = balance + :amount
  where id = :to_account_id;

-- @end create_transfer

-- Return the slug of every market.
-- @query get_market_slugs() ->* str
select slug from markets;

-- @query get_market_by_slug(slug: str) ->? Market
select
    id          -- :i64
  , slug        -- :str
  , kind        -- :str
  , title       -- :str
  , description -- :str
  , resolved_in -- :i64?
from
  markets
where
  slug = :slug;

-- Create a new market, return its id.
-- @query create_market(slug: str, kind: str, title: str, description: str) ->1 i64
insert into
  markets
  ( created_at
  , slug
  , kind
  , title
  , description
  )
  values
  ( strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
  , :slug
  , :kind
  , :title
  , :description
  )
  returning id;

-- Return the possible outcomes of a given market.
-- @query get_outcomes(market_id: i64) ->* Outcome
select id /* :i64 */, value /* :str */ from outcomes
  where market_id = :market_id
  order by id asc;

-- Insert an outcome, return its id.
-- @query create_outcome(market_id: i64, value: str) ->1 i64
insert into outcomes (market_id, value)
  values (:market_id, :value)
  returning id;

-- Return balances of all accounts related to the given market.
-- @query get_market_accounts(market_id: i64) ->* Account
select
    id       -- :i64
  , asset_id -- :i64
  , owner    -- :str
  , balance  -- :i64
from
  accounts
where
  market_id = :market_id;

-- @query create_realized_profit(
--   market_id: i64,
--   event_id: i64,
--   owner: str,
--   amount_in: i64,
--   amount_out: i64,
-- )
insert into realized_profits
  ( market_id
  , event_id
  , owner
  , amount_in
  , amount_out
  )
values
  ( :market_id
  , :event_id
  , :owner
  , :amount_in
  , :amount_out
  );

-- @query get_realized_profits(market_id: i64) ->* RealizedProfit
select
    owner      -- :str
  , amount_in  -- :i64
  , amount_out -- :i64
from
  realized_profits
where
  market_id = :market_id
order by
  amount_in - amount_out;
