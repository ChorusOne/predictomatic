-- Predictomatic -- A webapp for facilitating internal prediction markets
-- Copyright 2025 Chorus One

-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- A copy of the License has been included in the root of the repository.

-- Apapted from Hack-o-matic <https://github.com/ChorusOne/hackomatic>.
-- Copyright 2024 Chorus One, licensed Apache 2.0.

-- To be used with <https://github.com/ruuda/squiller>, you need commit
-- c83d64f644805e70 or later.

-- @query ensure_schema_versions_exists()
create table if not exists schema_versions
  ( version    integer not null
  , created_at text not null
  );

-- Create the schema.
--
-- We always have the latest version of the schema here: it's useful to have a
-- reference of what the tables and their columns are.
--
-- For migrations, we have separate queries below. We only do migrations up; to
-- go down, simply restore a back-up of your SQLite file before the migration.
-- @begin create_schema()
insert into
  schema_versions (version, created_at)
values
  (2, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));

create table accounts
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
create table events
( id          integer primary key
, created_at  text not null
, created_by  text not null
, description text not null
);

create table transfers
( id              integer primary key
, event_id        integer not null references events (id)
, from_account_id integer not null references accounts (id)
, to_account_id   integer not null references accounts (id)
, amount          integer not null
, check (amount > 0)
);

-- Records the balance of an account after an event affected the account.
-- Multiple transfers may be part of the same event, this table only records
-- the final balance after all transfers that are part of the event.
create table balances
( id            integer primary key
, account_id    integer not null references accounts (id)
, event_id      integer not null references events (id)
, balance_after integer not null
, unique (account_id, event_id)
);

create table markets
( id          integer primary key
, slug        text not null
, created_at  text not null
, kind        text not null
, title       text not null
, description text not null
, unique (slug)
, unique (title)
);

create table outcomes
( id          integer primary key
, market_id   integer not null references markets (id)
, value       text not null
  -- The event in which this outcome was picked as the resolution.
, resolved_in integer null references events (id)
, unique (market_id, value)
);

-- For resolved markets, we track the realized profit per participant.
-- Every profit links to a resolution event.
create table realized_profits
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

-- @begin migrate_schema_from_1_to_2()
insert into
  schema_versions (version, created_at)
values
  (2, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));

create table balances
( id            integer primary key
, account_id    integer not null references accounts (id)
, event_id      integer not null references events (id)
, balance_after integer not null
, unique (account_id, event_id)
);

insert into balances
  ( account_id
  , event_id
  , balance_after
  )
select
  accounts.id as account_id,
  events.id as event_id,
  coalesce((
    select sum(amount) from transfers
    where (transfers.event_id <= events.id) and (accounts.id = transfers.to_account_id)
  ), 0) - coalesce((
    select sum(amount) from transfers
    where (transfers.event_id <= events.id) and (accounts.id = transfers.from_account_id)
  ), 0)
from
  events,
  accounts
where
  exists (
    select 1 from transfers where
      (events.id = transfers.event_id)
      and (
          (accounts.id = transfers.from_account_id) or
          (accounts.id = transfers.to_account_id)
      )
  )
order by
  event_id asc,
  account_id asc;

-- @end migrate_schema_from_1_to_2()

-- @query get_schema_version() ->1 i64
select coalesce(max(version), 0) from schema_versions;

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
insert into transfers
  ( event_id
  , from_account_id
  , to_account_id
  , amount
  )
  values
  ( :event_id
  , :from_account_id
  , :to_account_id
  , :amount
  );

update accounts
  set   balance = balance - :amount
  where id = :from_account_id;

update accounts
  set   balance = balance + :amount
  where id = :to_account_id;

insert into
  balances
  ( event_id
  , account_id
  , balance_after
  )
values
  ( :event_id
  , :from_account_id
  , (select balance from accounts where id = :from_account_id)
  ),
  ( :event_id
  , :to_account_id
  , (select balance from accounts where id = :to_account_id)
  )
on conflict
  -- Overwrite a previous balance for this event, if any.
  -- We care only about the final balance per event.
  do update set balance_after = excluded.balance_after;

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
  , unixepoch() - unixepoch(created_at) as age_seconds -- :i64
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
select
    id          -- :i64
  , value       -- :str
  , resolved_in -- :i64?
from
  outcomes
where
  market_id = :market_id
order by
  id asc;

-- Insert an outcome, return its id.
-- @query create_outcome(market_id: i64, value: str) ->1 i64
insert into outcomes (market_id, value)
  values (:market_id, :value)
  returning id;

-- Return balances of all accounts related to the given market.
-- @query get_market_accounts(market_id: i64) ->* MarketAccount
select
    asset_id -- :i64
  , owner    -- :str
  , balance  -- :i64
from
  accounts
where
  market_id = :market_id;

-- Return ids of all global (not associated with a market) points accounts.
-- Excludes the system account.
-- @query get_user_points_accounts() ->* UserPointsAccount
select
    id -- :i64
from
  accounts
where
  -- Market id 0 means no market, asset id 0 is the points asset.
  (market_id = 0) and (asset_id = 0) and (owner <> 'SYSTEM');

-- Return balances of all global (not associated with a market) points accounts.
-- @query get_points_accounts() ->* PointsAccount
select
    owner    -- :str
  , balance  -- :i64
from
  accounts
where
  -- Market id 0 means no market, asset id 0 is the points asset.
  (market_id = 0) and (asset_id = 0);

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

-- @query create_resolution(outcome_id: i64, event_id: i64)
update outcomes set resolved_in = :event_id where id = :outcome_id;

-- Select all trades made up to and including the given event id,
-- or NULL to get the most recent trades.
-- @query get_trade_activity_until(limit: i64, event_id: i64?) ->* TradeActivity
select
    e.id as event_id           -- :i64
  , e.created_at as created_at -- :str
  , a.owner as user_email      -- :str
  , a.market_id as market_id   -- :i64
  , m.slug  as market_slug     -- :str
  , m.title as market_title    -- :str
  , o.value as outcome_label   -- :str
  , a.asset_id as asset_id     -- :i64
  , t.amount as amount_bought  -- :i64
from
  events e,
  transfers t,
  accounts a,
  markets m,
  outcomes o
where
  ((:event_id is null) or (e.id <= :event_id))
  and (e.description = 'Trade')
  and (t.event_id = e.id)
  and (t.to_account_id = a.id)
  and (a.owner <> 'SYSTEM')
  and (a.market_id = m.id)
  and (a.asset_id = o.id)
order by
  e.id desc
limit
  :limit;

-- Select all trades made in this market (see also `get_trade_activity_until`).
-- @query get_trade_activity_by_market(market_id: i64) ->* MarketTradeActivity
select
    e.id as event_id           -- :i64
  , e.created_at as created_at -- :str
  , a.owner as user_email      -- :str
  , o.value as outcome_label   -- :str
  , a.asset_id as asset_id     -- :i64
  , t.amount as amount_bought  -- :i64
from
  events e,
  transfers t,
  accounts a,
  outcomes o
where
  (e.description = 'Trade')
  and (t.event_id = e.id)
  and (t.to_account_id = a.id)
  and (a.owner <> 'SYSTEM')
  and (a.market_id = :market_id)
  and (a.asset_id = o.id)
order by
  e.id desc;
