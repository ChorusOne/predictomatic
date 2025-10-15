-- Predict-o-matic -- A webapp for facilitating internal prediction markets
-- Copyright 2025 Chorus One

-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- A copy of the License has been included in the root of the repository.

-- Apapted from Hack-o-matic <https://github.com/ChorusOne/hackomatic>.
-- Copyright 2024 Chorus One, licensed Apache 2.0.

-- To be used with https://github.com/ruuda/squiller v0.5.0

-- @begin ensure_schema_exists()
create table if not exists markets
( id          integer primary key
, created_at  string  not null
);

create table if not exists outcomes
( id        integer primary key
, market_id integer not null references markets (id)
, value     string not null
);

create table if not exist accounts
( id       integer primary key
, asset    integer not null references (outcomes)
, owner    string not null
, balance  integer not null
  -- Every user can have at most one account per asset.
, unique (token, owner)
);
-- @end ensure_schema_exists()
