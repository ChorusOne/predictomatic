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

## Applying the formula

The simple formulation of the Kelly Criterion, the gambling formula, states:

$$ f^* = q - \frac{1 - q}{b} $$

Where $f^*$ is the fraction of the bankroll to wager, and $b$ is the proportion
of the bet gained, the profit divided by the cost: $b = (x - C(x)) / C(x)$.
Plugging that in, we get:

$$ f^* = q - (1 - q)\frac{C(x)}{x - C(x)} $$

Dividing by the cost in numerator and denominator, we get:

$$ f^* = q - \frac{1 - q}{1 / \bar{p}(x) - 1.0} $$

where $\bar{p}(x) = C(x) / x$ is the average price per share.

## Maximizing log wealth

Where does the above formula come from? It’s the solution for a simplified case
to the general Kelly principle: maximize expected log wealth. Note that this is
not the same as maximizing the log of the expected wealth! (Which would be the
same as maximizing expected wealth, because log is monotonic.) Because
$\textup{log}(x)$ decreases rapidly as $x \to 0$, maximizing log wealth heavily
penalizes outcomes where we’d lose all our wealth. It values staying solvent
over recklessly maximizing wealth.

Framed this way, with our starting wealth $W$, we want to maximize:

$$ L(x) = \mathbb{E}(\textup{log} W_1) $$

where $W_1$ is the wealth after our bet. There are two scenarios:

 * If the outcome happens, $W_1 = W - C(x) + x$.
 * If the outcome does not happen, $W_1 = W - C(x)$.

The former happens with probability $q$, the latter with probability $1 - q$.
In both cases we spend $C(x)$ to buy $x$ shares. In the positive outcome, each
share pays out $1$ for a total of $x$, in the negative outcome we get nothing.
In other words, we need to maximixe:

$$ L(x) = q \textup{log}(W - C(x) + x) + (1 - q)\textup{log}(W - C(x)) $$

We maximize this by setting the $dL/dx$ equal to $0$. Recall that $dC/dx = p$,
so we get:

$$ L'(x) = q \frac{1 - p(x)}{W - C(x) + x} - (1 - q)\frac{p(x)}{W - C(x)} = 0 $$

Rearranging the terms, we get:

$$ \frac{p(x)}{1 - p(x)} = \frac{q}{1 - q} \cdot \frac{W - C(x)}{W - C(x) + x} $$

We can draw a few interesting conclusions from this:

**Infinite wealth limit.**<br>
Say $W >> x$, then:

```math
\frac{W - C(x)}{W - C(x) + x} \approx 1
\implies \frac{p(x)}{1 - p(x)} \approx \frac{q}{1 - q}
\implies p(x) \approx q
```

In other words, if the bet is small compared to our wealth, we should buy until
the marginal price matches $q$. This is the case where we can forget about
Kelly, and we’re just maximizing expected wealth. Keep buying the shares while
they are undervalued.

**Finite wealth limit.**<br>
When the above approximation does not apply, we have:

```math
\frac{W - C(x)}{W - C(x) + x} < 1
\implies \frac{p(x)}{1 - p(x)} < \frac{q}{1 - q}
```

So we should stop buying shares _before_ $p(x)$ reaches $q$.

To solve this for $p(x)$,
let’s simplify the notation a bit.
We’ll write $p$ for $p(x)$ and $W$ for $W - C(x)$.
Then we have:

```math
\frac{p}{1 - p} = \frac{q}{1 - q} \cdot \frac{W}{W + x} \\
\implies p = \frac{(1 - p)qW}{(1 - q)(W + x)} \\
\implies p(1 - q)(W + x) = (1 - p)qW \\
\implies p(qW + (1 - q)(W + x)) = qW \\
\implies p = \frac{qW}{qW + (1 - q)(W + x)} \\
\implies p = \frac{qW}{W + (1 - q)x}
```
