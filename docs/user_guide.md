# Help

<!-- Although the extension is .md so we can get this document to render on e.g.
GitHub, it is not _really_ Markdown, it's html with limited support for headings,
see also src/routes/help.rs.

In this document, please don't fill up the lines to 80 columns,
instead add line breaks after periods or commas, to keep the diff sensible.
-->

Predict-o-matic facilitates prediction markets.
Prediction markets are a tool for aggregating information about future events.
This manual provides a short introduction to prediction markets,
and how they work in Predict-o-matic.

## Markets

Predict-o-matic hosts <em>markets</em>.
Every market has two or more <em>outcomes</em>.
For example,
a market might ask “Will Ethereum be worth more than $4000 on 2026-01-01?”
with outcomes “Yes” and “No”.
Users can trade <em>outcome shares</em> in these outcomes,
and the price of a share corresponds to
the probability that the outcome will happen.
In this way,
Predict-o-matic aggregates input from users
into a single market prediction.

## Points

Predict-o-matic simulates a virtual economy
where the main currency is <em>points</em>,
denoted with the ‘$’-symbol.
Every new user starts out with some points.
You have $POINTS of them right now,
which you can also see in the top-right corner of the screen.
Prediction markets are zero-sum games
where points flow from those who make bad predictions
to those who make good predictions.

## Outcome shares

As long as the market is open,
outcome shares have a price between $0 and $1.
When a market resolves,
shares in the outcome that it resolved to pay out $1,
and other shares become worthless.
If you have a share in every outcome,
then you will receive $1 at resolution regardless of how the market resolves,
therefore a basket with one share in every outcome is worth $1.

## Depositing

When you deposit points into a market,
you lock up the points,
and you get an equal amount of shares in every outcome in return.
For example,
if you deposit $10 into the example market described earlier,
you get 10 Yes + 10 No.
This is a neutral position:
your proceeds do not depend on how the market resolves.
To take a non-neutral position,
you can trade the Yes and No shares.

Deposited points are locked until resolution,
withdrawing from a market is not possible.
In other words,
you cannot realize profits until the market resolves.
This ensures that the market can be dissolved when it is unclear how it should resolve.
Every participant gets their points back,
and we pretend that no trades ever happened.
Although it is not possible to withdraw,
you can exit a position and take profits by trading
until you have an equal amount of every outcome share.

## Trading

If the current price of Yes
