## v0.3.0

Released 2025-12-31.

 * Depositing is no longer a separate step. Instead, when you trade, that trade
   will automatically includ a deposit when needed.
 * The trade interface now shows the trade volume in points. Behind the scenes,
   trades still only swap outcome shares for outcome shares, not for points.
 * A new _bet sizing help_ section helps to select (fractional) Kelly bets, and
   to exit to an outcome-neutral position.
 * The dark theme now has slightly lower contrast to make it easier on the eye.

## v0.2.0

Released 2025-12-18.

For users:

 * Improve the sort order on the index page to surface interesting markets first.
 * Exclude the system user from liquidity computations.
 * Clarify the manual.

For admins:

 * Add a way for admins to distribute a bonus.

Internal changes:

 * Store new account balances at every transfer. This is in preparation for
   graphing a prediction over time.

## v0.1.0

Released 2025-10-20.

Initial release.
