# The Kelly Criterion in Predicton Markets

The simple form of the [Kelly Criterion][kelly] tells us how much of our
bankroll to wager when we can make a bet with a win probability $q$, and a given
proportional gain. This is very relevant for a prediction market: even if we
think the market is mispriced, investing our entire net worth into correcting
the price is risky, and can lead to financial ruin. A bet might have a high
expected return, but we can still lose! Even if in practice [you never go full
Kelly][full-kelly], having access to Kelly fraction can help us size bets, and
to avoid being reckless.

[kelly]:      https://en.wikipedia.org/wiki/Kelly_criterion
[full-kelly]: https://www.lesswrong.com/posts/BZ6XaCwN4QGgH9CxF/the-kelly-criterion

## Framing the problem

Suppose we want to participate in a market, and buy a given outcome share.
This outcome currently has marginal price $p_0$, i.e. the market prediction is
$p_0$. Our belief is that the outcome will in fact happen with probability
$q > p_0$. This means we can buy points at a discount — at least, in expected
value. How many shares should we buy?

Let $p(x)$ be the marginal price per share when we buy $x$ shares, and let
$C(x)$ be the total cost of buying $x$ shares. (So $p = dC/dx$.)

If we wanted to maximize expected profit, and we had an infinite amount of
points to spend, we should buy while $`p(x) < q`$: as long as the expected
payout of a share is higher than its cost.

In practice, we don’t have an infinite amount of points to spend. Even if we can
afford to buy $x$ shares such that $p(x) = q$, it would be unwise to risk our
entire wealth $W$.

## Applying the Gambling Formula

The simple formulation of the Kelly Criterion, the gambling formula, says:

$$ f^* = q - \frac{1 - q}{b} $$

Where $f^*$ is the fraction of the bankroll to wager, and $b$ is the proportion
of the bet gained, the profit divided by the cost: $b = (x - C(x)) / C(x)$.
Plugging that in, we get:

$$ f^* = q - (1 - q)\frac{C(x)}{x - C(x)} $$

Dividing by the cost in numerator and denominator, we get:

$$ f^* = q - \frac{1 - q}{1 / \bar{p}(x) - 1.0} $$

where $\bar{p}(x) = C(x) / x$ is the average price per share.

