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

## Trading

You can trade in a market by moving the probability slider
and then clicking the ‘Trade’ button to confirm the trade.
When you trade,
you buy outcome shares from an <em>automated market maker</em>
(see <a href="#automated-market-maker">below</a>).
There are two ways to pay for that order.
If you already own outcome shares in the opposite outcome,
you can sell those.
If you have not participated in the market before,
you can <em>deposit</em> to create new outcome shares.
The system selects the optimal way to finance the order for you.
It will sell outcome shares before spending liquid points.

When you trade,
this changes the market price of outcome shares,
and therefore the market prediction.
The market price is the <em>marginal</em> price of an outcome share:
the price you’d pay if you bought a tiny amount.
The more shares you buy,
the more their price goes up.
This means that the <em>average</em> price you pay per share,
is lower than the new marginal price after your trade.

## Making a profit

When your belief about an outcome differs from the market price,
you can trade for an expected profit.
For example,
if you believe that there is an 80% probability that the market will resolve ‘Yes’,
that corresponds to a price of $0.80.
If the current market price for Yes shares is $0.60,
they are effectively on discount to you.
Suppose you spend $7 to buy 10 Yes shares,
and that trade moves the market price from $0.60 to $0.80.
You believe that when the market resolves,
with 80% probability you get $10 (a profit of $3);
with 20% probability you get $0 (a loss of $7).
Your expected profit is therefore 0.8&nbsp;×&nbsp;$3 – 0.2&nbsp;×&nbsp;$7 = $1.

Note that this $1 “expected profit” is merely a <em>statistical tool</em>.
There is not actually an outcome where you make $1 profit!
You either make a $3 profit, or a $7 loss.
In particular,
even though your expected profit is positive,
<em>you can still make a loss</em>.

## Bet sizing

The trade that maximizes your expected profit is to buy shares
until their marginal price matches what you believe is the true probability.
This strategy is optimal when you have infinite money to spend,
but it’s reckless when your bank account is finite.
As shown <a href="#making-a-profit">above</a>,
even when your expected profit is positive,
you can be unlucky and suffer a loss.
This can lead to <em>ruin</em>:
when you are bankrupt, you don’t get to trade again.

To avoid ruin,
rather than maximizing our expected wealth,
we can maximize our expected <em>log wealth</em>.
This has several nice properties.
In particular, the logarithm goes to –∞ as wealth goes to 0,
so this approach penalizes bets that risk bankruptcy
while still encouraging wealth growth.
Maximizing log wealth is called the <em>Kelly Criterion</em>,
and the optimal amount to bet is called the <em>Kelly bet</em>.
Kelly helps us stay solvent:
a participant who consistently bets more than the Kelly bet,
will eventually bankrupt themselves with probability 1.

Kelly helps us avoid ruin, but it’s still risky.
Standard investment advice goes:
<strong>never go full Kelly</strong>.
Kelly only <em>barely</em> avoids ruin,
and the Kelly bet depends on your probability estimate.
When your estimates are off,
you are taking on more risk than you realize.

To help you size your bet,
the trade interface contains <em>bet sizing help</em> in the sidebar.
You enter your belief for the probability that the market resolves Yes
by moving the trade slider,
and the bet sizing help computes the trade that maximizes your log wealth
under that belief.
You can click the different fractions to select that trade.
(You still need to click the ‘Trade’ button to execute the trade.)
The bet sizing help takes your existing outcome shares into account.
For this reason, it sometimes recommends trading in the opposite direction:
if you can greatly reduce your worst-case loss
at a very small cost to your best-case profit,
that can be a sensible thing to do.

## Deposits

To participate in a market, you deposit points.
This happens automatically when you trade,
as described in the <a href="#trading">trading section</a>.
By depositing points, you create an equal amount of shares in every outcome.
For instance,
if you deposit $10 into the example market described earlier,
you get 10 Yes + 10 No.
When the market resolves as Yes,
that $10 is paid out to the holders of the Yes shares;
when it resolves No the $10 is paid to No holders.

Deposited points are locked until resolution,
withdrawing from a market is not possible.
In other words,
you cannot realize profits until the market resolves.
This ensures that the market can be dissolved when it is unclear how it should resolve.
When a market dissolves,
every participant gets their deposit back,
as if no trades ever happened.

Although it is not possible to withdraw,
you can exit a non-neutral position and take profits by trading
until you have an equal amount of every outcome share.
Such an outcome-neutral position pays out the same amount
regardless of how the market resolves.
The <em>bet sizing help</em> section in the sidebar includes a button
that computes the appropriate trade to take an outcome-neutral position.

## Automated market maker

When you trade,
you trade against an automated market maker that is managed by the system.
The market maker ensures that you can always trade,
and it provides a way to subsidize the market.
Prediction markets are zero-sum.
If you are going to make money here,
<em>somebody</em> has to be losing money.
When you are the only participant in a market,
or when all participants bet in the same direction,
it’s the system that takes the other side of the bet.

When you trade against the automated market maker,
you <em>swap</em> outcome shares.
For example, you buy 5 Yes shares, and you pay for that with 10 No shares.
Even though only outcome shares change hands,
every swap implies a price per share,
and therefore a probability.
The ratio of No:Yes corresponds to the odds of a positive outcome.
Odds of 10:5 mean a probability of 10/15,
so about 67% probability that ‘Yes’ will happen,
and 33% that ‘No’ will happen.
In other words, the average price you paid was $0.67 per Yes share.
When you move the slider to trade,
the user interface displays this average price in points for your convenience,
even though only outcome shares change hands.
