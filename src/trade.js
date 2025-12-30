// How difficult is it to change the market price?
// For a value of 41.5, one can change the market from 50/50 to 85% by selling
// 50 YES shares, half the starting balance. I think that's reasonable?
const marketB = 41.5;

// This value remains invariant under swaps against the pool, so we can compute
// it once at the start.
const invariant = systemBalance
    .map(lk => Math.exp(-lk / marketB))
    .reduce((acc, p) => acc + p, 0.0);

// The maximum probability for outcome 0, is when we'd sell all user shares
// to the pool.
const max0 = systemBalance[0] + userBalance[0] + userLiquidPoints;
const pMin = Math.exp(-max0 / marketB) / invariant;

// Same for outcome 1, and for a binary question, that gives us the maximum
// probability we can afford to buy.
const max1 = systemBalance[1] + userBalance[1] + userLiquidPoints;
const pMax = 1.0 - Math.exp(-max1 / marketB) / invariant;

const canTrade = pMax - pMin > 0.001;

// When we compute the Kelly bet when moving the slider, we store it here, so
// that when the user clicks one of the rows in the bet sizing help, we can
// apply from here. We also store the probability that we need to trade to.
let kellyBet = null;
let kellyProbability = null;

function getProbability(balance) {
    const ps = balance.map(lk => Math.exp(-lk / marketB));
    return ps[0] / invariant;
}

// Return new balances and trade volume if we traded until probability `p`.
function getTrade(p) {
    // If we trade up to `p`, d0, d1 are the shares that change hands.
    const new0 = -Math.log(invariant * p) * marketB;
    const new1 = -Math.log(invariant * (1.0 - p)) * marketB;
    const d0 = new0 - systemBalance[0];
    const d1 = new1 - systemBalance[1];

    // So these are our new balances, if we allow them to go negative.
    const b0a = userBalance[0] - d0;
    const b1a = userBalance[1] - d1;

    // Share balances can't go negative though, so we may need to deposit.
    const deposit = -Math.min(b0a, b1a, 0.0);
    const b0 = b0a + deposit;
    const b1 = b1a + deposit;

    return {
        probability: p,
        userDelta: [-d0, -d1],
        userBalance: [b0, b1],
        deposit: deposit,
    };
}

function getTradeDetails(trade) {
    // We can compute the average cost from the ratio: the sum of the prices
    // is 1, so solving yields price = 1 / (ratio + 1). In other words, the
    // ratio of the prices is the odds, and we convert the odds back to a
    // probability. When both amounts are 0, the ratio does not exist, and we
    // get NaN here, but in that case we don't use the value below.
    const ratio = -trade.userDelta[0] / trade.userDelta[1];
    const price0 = 1.0 / (ratio + 1.0);
    let result = null;

    do {
        // We don't want to let the user do tiny trades. After rounding to 2
        // decimals, we must trade more than 0.01 of each asset.
        if (trade.userDelta[0] > 0.015 && trade.userDelta[1] < 0.015) {
            result = {
                assetBuy: 0,
                assetSell: 1,
                amountBuy: trade.userDelta[0],
                amountSell: -trade.userDelta[1],
                sharePricePoints: price0,
                valid: true,
            };
            break;
        }

        if (trade.userDelta[1] > 0.015 && trade.userDelta[0] < 0.015) {
            result = {
                assetBuy: 1,
                assetSell: 0,
                amountBuy: trade.userDelta[1],
                amountSell: -trade.userDelta[0],
                sharePricePoints: 1.0 - price0,
                valid: true,
            };
            break;
        }

        // If the slider goes past the current market state, there is nothing to
        // buy or sell, so we don't have a trade. But if we would hide the table,
        // that's visually very jarring, so instead we pretend to buy 0.0 of
        // asset 0.
        result = {
            assetBuy: 0,
            assetSell: 1,
            amountBuy: 0.0,
            amountSell: 0.0,
            // If there is no swap, so no implied share price, then we can still
            // use the market probability, which is the marginal price.
            sharePricePoints: trade.probability,
            valid: false,
        };
    }
    while (false);

    // The volume for the purpose of Kelly computations etc. on this page, is
    // including deposit. It's about what we buy, not about how we finance it.
    result.volumePoints = (result.amountBuy + trade.deposit) * result.sharePricePoints;

    return result;
}

// Return the expected log wealth if we trade up to marginal probability `p`,
// but we believe the true probability of outcome 0 is `q`.
//
// Returns NaN for cases where we can't afford the trade, because the wealths
// would be negative, so log(wealth) does not exist.
function expectedLogWealth(p, q) {
    const trade = getTrade(p);
    const b0 = trade.userBalance[0];
    const b1 = trade.userBalance[1];
    const bp = userLiquidPoints - trade.deposit;

    // The log wealth for both outcome 0 and 1.
    const lw0 = Math.log(bp + b0);
    const lw1 = Math.log(bp + b1);

    return q * lw0 + (1.0 - q) * lw1;
}

// Given our true probability `q` for outcome 0, return the price `p` that we
// should trade to to maximize our expected log wealth (the Kelly Criterion).
function maximizeExpectedLogWealth(q) {
    const clamp = (x, min, max) => Math.max(min, Math.min(x, max));
    // Iterate performs one Newton-Raphson iteration to get closer to a root of
    // the first derivative of `expectedLogWealth`. We estimate the first and
    // second derivative numerically.
    const iterate = p => {
        const pp = p;
        const p0 = Math.max(p - 0.01, (p + pMin) * 0.5, pMin);
        const p1 = Math.min(p + 0.01, (p + pMax) * 0.5, pMax);

        // Evaluate the log wealth at three points.
        const lw0 = expectedLogWealth(p0, q);
        const lwp = expectedLogWealth(pp, q);
        const lw1 = expectedLogWealth(p1, q);

        // The three points give us two derivatives, and those give us one
        // second derivative.
        const d0 = (lwp - lw0) / (pp - p0);
        const d1 = (lw1 - lwp) / (p1 - pp);
        const dd = (d1 - d0) / ((p1 - p0) * 0.5);
        const d = (d0 + d1) * 0.5;

        // The standard formula has a step size of 1, but we know that the log
        // wealth goes to zero at p=0 and p=1. If the optimum is at say p=0.97,
        // then if we overshoot, we end up in this steep region of the graph
        // where it takes a few iterations to recover. So when we get close to
        // the edge, take smaller steps.
        const stepSize = (p < 0.20 || p > 0.80) ? 0.5 : 1.0;

        // We don't want the suggestion to go all the way to pMin or pMax even
        // when the optimum is there, because then we have no headroom to
        // compute the derivative numerically.
        const pNew = clamp(p - (d / dd), pMin + 0.001, pMax - 0.001);
        return stepSize * pNew + (1.0 - stepSize) * p;
    };

    // If the optimium is somewhere between 0.1 and 0.9, it converges very
    // quickly, 5 iterations is plenty. But if the optimum is closer to the
    // extremes, it converges more slowly. 15 iterations works either way.
    let p = 0.5;
    for (let i = 0; i < 15; i++) p = iterate(p);

    return p;
}

// Return the probability `p` that we should trade to, so that at the end we
// have an equal number of shares in every outcome.
function getNeutralPositionProbability() {
    const clamp = (x, min, max) => Math.max(min, Math.min(x, max));
    // Iterate performs one Newton-Raphson iteration; see also
    // `maximizeExpectedLogWealth` above.
    const iterate = p => {
        const p0 = Math.max(p - 0.001, (p + pMin) * 0.5, pMin);
        const p1 = Math.min(p + 0.001, (p + pMax) * 0.5, pMax);

        // Newton-Raphson finds root, in this case we want a root of the
        // difference between balances, so we only need to numerically estimate
        // the first derivative.
        const t0 = getTrade(p0);
        const t1 = getTrade(p1);
        const v0 = t0.userBalance[0] - t0.userBalance[1];
        const v1 = t1.userBalance[0] - t1.userBalance[1];
        const vm = (v0 + v1) * 0.5;
        const dv = (v1 - v0) / (p1 - p0);
        return clamp(p - (vm / dv), pMin + 0.001, pMax - 0.001);
    };

    // 15 iterations is enough to converge to more than 10 decimal digits.
    // Event 7 seems to be enough but it's fast anyway.
    let p = 0.5;
    for (let i = 0; i < 15; i++) p = iterate(p);

    return p;
}

// Return the PnL that we would realize when we exit to the neutral position.
//
// This is less than the "Unrealized PnL" that the "Your balance" table displays
// due to slippage of the trade.
function getNeutralPositionProfit() {
    const p = getNeutralPositionProbability();
    const base = getTrade(p);
    const trade = getTradeDetails(base);
    return base.userBalance[0] - userDeposited;
}

// Prepare a trade offer, such that after the trade, the market's implied
// probability is `p`.
function updateTradeWidget(p) {
    const baseTrade = getTrade(p);
    const trade = getTradeDetails(baseTrade);

    const offerElem = document.getElementById("trade-offer");
    const offerTable = offerElem.getElementsByTagName("table")[0];

    const amountBuy = trade.amountBuy;
    const amountSell = trade.amountSell;
    const labelBuy = assetLabels[trade.assetBuy];
    const labelSell = assetLabels[trade.assetSell];
    const sharePricePoints = trade.sharePricePoints;

    // The `amountBuy` and `amountSell` so far are against the system. If the
    // user does not have enough shares to sell, we need to make an additional
    // deposit. From the deposit we get both outcome shares, so in a sense we
    // "bought" more. Also, we don't want to double-count this deposit as shares
    // "sold". So adjust for deposit to get an "effective" amount bought/sold.
    const deposit = baseTrade.deposit;
    const effectiveBuy = amountBuy + deposit;
    const effectiveSell = amountSell - deposit;
    const costPointsBuy = effectiveBuy * sharePricePoints;
    const costPointsSell = effectiveSell * (1.0 - sharePricePoints);

    var sellRow = effectiveSell <= 0.0 ? "" : `
    <tr>
        <td>Sell</td>
        <td class="num">$\u{200a}${costPointsSell.toFixed(2)}</td>
        <td>worth of</td>
        <td>${labelSell},</td>
        <td class="num">${effectiveSell.toFixed(2)}</td>
        <td>shares at</td>
        <td class="num">$\u{200a}${(1.0 - sharePricePoints).toFixed(2)} per share</td>
        <td class="filler"></td>
    </tr>
    `;

    var depositRow = deposit <= 0.0 ? "" : `
    <tr>
      <td>Pay</td>
      <td class="num">$\u{200a}${deposit.toFixed(2)}</td>
      <td colspan="6"> from your liquid points</td>
    </tr>
    `;


    offerTable.innerHTML = `
    <tr>
        <td>Buy</td>
        <td class="num strong">$\u{200a}${costPointsBuy.toFixed(2)}</td>
        <td>worth of</td>
        <td><strong>${labelBuy}</strong>,</td>
        <td class="num">${effectiveBuy.toFixed(2)}</td>
        <td>shares at</td>
        <td class="num"><strong>$\u{200a}${sharePricePoints.toFixed(2)}</strong> per share</td>
        <td class="filler"></td>
    </tr>
    ${sellRow}
    ${depositRow}
    `;

    if (trade.valid) {
        document.trade_form.asset_in.value = assetIds[trade.assetSell];
        document.trade_form.asset_out.value = assetIds[trade.assetBuy];
        document.trade_form.amount_in.value = trade.amountSell.toFixed(2);
        // Build in 2% slippage tolerance.
        document.trade_form.min_out.value = (trade.amountBuy * 0.98).toFixed(6);
        document.trade_form.max_deposit.value = deposit.toFixed(6);
    }

    // For trades that include a large deposit, ask the user to confirm.
    const isRisky = deposit > userLiquidPoints * 0.5;
    const warningElement = document.getElementById("trade-warning");
    const warningAckBox = document.getElementById("trade-warning-acknowledge");
    if (isRisky) {
        warningElement.classList.add("enabled");
    } else {
        warningElement.classList.remove("enabled");
    }

    const canSubmit = trade.valid && (warningAckBox.checked || !isRisky);
    const submitButton = document.getElementById("trade-submit");
    submitButton.disabled = !canSubmit;
}

function updateBetSizingHelp(q) {
    // Update the global variables with the Kelly bet for what we believe
    // is the true probability of outcome 0.
    const pKelly = maximizeExpectedLogWealth(q);
    kellyBet = getTradeDetails(getTrade(pKelly));
    kellyProbability = pKelly;

    const labelBuy = assetLabels[kellyBet.assetBuy];
    const kellyVolume = kellyBet.volumePoints;

    document.getElementById("sizing-help-kelly").innerHTML = `
    <tr title="Your estimate of the probability that the market resolves ${assetLabels[0]},
for the purpose of the Kelly computation.">
        <td>Belief ${assetLabels[0]}</td>
        <td class="num">${(q * 100.0).toFixed(1)}%</td>
    </tr>
    <tr title="The trade that maximizes your expected log-wealth.">
        <td>Kelly bet</td>
        <td class="num">$\u200a${kellyVolume.toFixed(2)} of ${labelBuy}</td>
    </tr>
    `;
}

// Given a fraction, return the probability that we should trade to so that the
// trade volume is the given fraction of the Kelly bet.
function getFractionalKellyProbability(fraction) {
    const targetVolume = kellyBet.volumePoints * fraction;

    // We know the volume of Full Kelly, and the probability to which we
    // need to set the slider to make that trade, but if we want a fraction
    // of that, we want a fraction of the *volume* and not a fraction of the
    // probability movement. There is probably a way to solve it exactly but
    // I am lazy so we'll just binary search for it.
    let p0 = getProbability(systemBalance);
    let p1 = kellyProbability;

    if (fraction < 1.0) {
        let v0 = 0.0;
        let v1 = kellyBet.volumePoints;

        // 20 iterations is enough for it to converge to match the target volume
        // to 4 decimals.
        for (let i = 0; i < 20; i++) {
            let pm = (p0 + p1) * 0.5;
            let trade = getTradeDetails(getTrade(pm));
            let vm = trade.volumePoints;

            if (vm > targetVolume) {
                p1 = pm;
            } else {
                p0 = pm;
            }
        }
    }

    return p1;
}

function initialize() {
    const widget = document.getElementById("trade-widget");
    const knob = widget.getElementsByClassName("knob")[0];
    const hr = widget.getElementsByTagName("hr")[0];
    const tMarket = widget.getElementsByClassName("tmarket")[0];
    const pMarket = widget.getElementsByClassName("pmarket")[0];
    const tUser = widget.getElementsByClassName("tuser")[0];
    const pUser = widget.getElementsByClassName("puser")[0];

    if (canTrade) {
        knob.classList.remove("disabled");
    }

    var startX = 0.0;
    var startY = 0.0;
    var widgetRect = widget.getClientRects()[0];
    var knobRect = knob.getClientRects()[0];
    var hrRect = hr.getClientRects()[0];
    var selectedProbability = getProbability(systemBalance);

    const updateRects = () => {
        widgetRect = widget.getClientRects()[0];
        knobRect = knob.getClientRects()[0];
        hrRect = hr.getClientRects()[0];
    };

    const setPos = (elem, p) => {
        const rect = elem.getClientRects()[0];
        const xRel = p * hrRect.width - rect.width * 0.5;
        const xAbs = hrRect.left - widgetRect.left + xRel;
        elem.style.left = `${xAbs}px`;
    };
    const setPercentage = (tElem, pElem, p) => {
        setPos(tElem, p);
        setPos(pElem, p);
        pElem.innerText = `${(p * 100.0).toFixed(1)}%`;
    };
    const positionSlider = (p) => {
        setPos(knob, p);
        setPercentage(tUser, pUser, p);
    };
    const onDragMove = (event) => {
        const dx = event.clientX - startX;
        const rx = (knobRect.left + 0.5 * knobRect.width - hrRect.left + dx) / hrRect.width;
        // Clamp to [0.1%, 99.9%], or what the user can afford, whichever is
        // narrower.
        const min = Math.max(pMin, 0.001);
        const max = Math.min(pMax, 0.999);
        const p = Math.max(min, Math.min(max, rx));
        selectedProbability = p;
        positionSlider(selectedProbability);
        updateTradeWidget(selectedProbability);
        updateBetSizingHelp(selectedProbability);
    };
    const onDragEnd = (event) => {
        document.removeEventListener("mouseup", onDragEnd);
        document.removeEventListener("touchend", onDragEnd);
        document.removeEventListener("mousemove", onDragMove);
        document.removeEventListener("touchmove", onDragMove);
    };
    const onDragStart = (event) => {
        updateRects();
        startX = event.clientX;
        startY = event.clientY;
        document.addEventListener("mouseup", onDragEnd);
        document.addEventListener("touchend", onDragEnd);
        document.addEventListener("mousemove", onDragMove);
        document.addEventListener("touchmove", onDragMove);
    };
    const onResize = (event) => {
        updateRects();
        const p = getProbability(systemBalance);
        setPercentage(tMarket, pMarket, p);
        setPercentage(tUser, pUser, selectedProbability);
        if (isOpen) positionSlider(selectedProbability);
    };

    // For trades that spend a large amount of the available balance, we have
    // a checkbox to acknowledge that. If the user toggles it, we have to also
    // change the enabled state of the trade button, the simplest way is to just
    // refresh the entire trade widget.
    const onToggleAck = (event) => updateTradeWidget(selectedProbability);

    if (canTrade) {
        knob.addEventListener("mousedown", onDragStart);
        knob.addEventListener("touchstart", onDragStart);

        const warningAckBox = document.getElementById("trade-warning-acknowledge");
        warningAckBox.addEventListener("change", onToggleAck);

        document.onClickKelly = (fraction) => {
            selectedProbability = getFractionalKellyProbability(fraction);
            positionSlider(selectedProbability);
            updateTradeWidget(selectedProbability);
        };

        document.onClickNeutral = () => {
            selectedProbability = getNeutralPositionProbability();
            positionSlider(selectedProbability);
            updateTradeWidget(selectedProbability);
        };

        // On the button that selects the neutral position, update the label
        // to also say how much profit or loss that will realize.
        if (userDeposited > 0.0) {
            const neutralPnL = getNeutralPositionProfit();
            const neutralLabel =
                neutralPnL >= 0.0
                ? `${neutralPnL.toFixed(2)} profit`
                : `${(-neutralPnL).toFixed(2)} loss`;

            document.getElementById("button-neutral").innerText =
                `Neutral, $\u200a${neutralLabel}`;
        }
    }

    const p = getProbability(systemBalance);
    setPercentage(tMarket, pMarket, p);
    setPercentage(tUser, pUser, p);
    positionSlider(p);
    updateBetSizingHelp(p);

    window.addEventListener("resize", onResize);
}

document.addEventListener("DOMContentLoaded", initialize);
