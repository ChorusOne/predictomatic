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

function getProbability(balance) {
    const ps = balance.map(lk => Math.exp(-lk / marketB));
    return ps[0] / invariant;
}

// Return the expected log wealth if we trade up to marginal probability `p`,
// but we believe the true probability of outcome 0 is `q`.
//
// Returns NaN for cases where we can't afford the trade, because the wealths
// would be negative, so log(wealth) does not exist.
function expectedLogWealth(p, q) {
    // If we trade up to `p`, d0, d1 are the shares that change hands.
    const new0 = -Math.log(invariant * p) * marketB;
    const new1 = -Math.log(invariant * (1.0 - p)) * marketB;
    const d0 = new0 - systemBalance[0];
    const d1 = new1 - systemBalance[1];

    // So these are our new balances, if we allow them to go negative.
    const b0a = userBalance[0] - d0;
    const b1a = userBalance[1] - d1;

    // Share balances can't go negative though, so we may need to deposit.
    const deposit = Math.min(b0a, b1a, 0.0);
    const b0 = b0a + deposit;
    const b1 = b1a + deposit;
    const bp = userLiquidPoints - deposit;

    // From those we can get the log wealth for both outcomes.
    const lw0 = Math.log(bp + b0);
    const lw1 = Math.log(bp + b1);

    return q * lw0 + (1.0 - q) * lw1;
}

// Given our true probability `q` for outcome 0, return the price `p` that we
// should trade to to maximize our expected log wealth (the Kelly Criterion).
function maximizeExpectedLogWealth(q) {
    // Iterate performs one Newton-Raphson iteration to get closer to a root of
    // the first derivative of `expectedLogWealth`. We estimate the first and
    // second derivative numerically.
    const iterate = p => {
        const p0 = Math.max(p - 0.02, p * 0.5, pMin);
        const p1 = Math.min(p + 0.02, (p + 1.0) * 0.5, pMax);
        const ph = (p0 + p1) * 0.5;
        const dp = (p1 - p0) * 0.5;

        // Evaluate the log wealth at three points.
        const lw0 = expectedLogWealth(p0, q);
        const lwh = expectedLogWealth(ph, q);
        const lw1 = expectedLogWealth(p1, q);

        // The three points give us two derivatives, and those give us one
        // second derivative.
        const d0 = (lwh - lw0) / dp;
        const d1 = (lw1 - lwh) / dp;
        const dd = (d1 - d0) / dp;
        const d = (d0 + d1) * 0.5;

        // The standard formula has a step size of 1, but we know that the log
        // wealth goes to zero at p=0 and p=1. If the optimum is at say p=0.97,
        // then if we overshoot, we end up in this steep region of the graph
        // where it takes a few iterations to recover. So when we get close to
        // the edge, take smaller steps.
        const stepsize = (p < 0.20 || p > 0.80) ? 0.33 : 1.0;

        console.log(p, lwh);

        return Math.max(pMin, Math.min(pMax, p - stepsize * (d / dd)));
    };

    // If the optimium is somewhere between 0.1 and 0.9, it converges very
    // quickly, 5 iterations is plenty. But if the optimum is closer to the
    // extremes, it converges more slowly. 15 iterations works either way.
    let p = 0.5;
    for (let i = 0; i < 15; i++) p = iterate(p);

    return p;
}

// Prepare a trade offer, such that after the trade, the market's implied
// probability is `p`.
function getTrade(p) {
    const new0 = -Math.log(invariant * p) * marketB;
    const new1 = -Math.log(invariant * (1.0 - p)) * marketB;
    const d0 = new0 - systemBalance[0];
    const d1 = new1 - systemBalance[1];

    var trade = null;

    // The deltas are the delas in the *pool balance*, not the deltas in the
    // user's balance, but the trade is from the point of view of the user.
    if (d0 > 0.01) {
        trade = {
            assetBuy: 1,
            assetSell: 0,
            amountBuy: -d1,
            amountSell: d0,
            valid: true,
        };
    } else if (d1 > 0.01) {
        trade = {
            assetBuy: 0,
            assetSell: 1,
            amountBuy: -d0,
            amountSell: d1,
            valid: true,
        };
    } else {
        // If the slider goes past the current market state, there is nothing to
        // buy or sell, so we don't have a trade offer. But if we would hide the
        // table, that's visually very jarring, so instead we pretend to buy 0.0
        // of asset 0.
        trade = {
            assetBuy: 0,
            assetSell: 1,
            amountBuy: 0.0,
            amountSell: 0.0,
            valid: false,
        };
    }

    const offerElem = document.getElementById("trade-offer");
    const offerTable = offerElem.getElementsByTagName("table")[0];

    const amountBuy = trade.amountBuy;
    const amountSell = trade.amountSell;
    const labelBuy = assetLabels[trade.assetBuy];
    const labelSell = assetLabels[trade.assetSell];
    const ratio = trade.amountBuy / trade.amountSell;

    // We can compute the average cost from the ratio: the sum of the prices
    // is 1, so solving yields price = 1 / (ratio + 1). In other words, the
    // ratio of the prices is the odds, and we convert the odds back to a
    // probability. However, when both amounts are 0, the ratio does not exist.
    // That case happens when the slider is at the market probability, so then
    // we still have a price: the current market price.
    const sharePricePoints = trade.valid ? (1.0 / (ratio + 1.0)) : p;

    // The `amountBuy` and `amountSell` so far are against the system. If the
    // user does not have enough shares to sell, we need to make an additional
    // deposit. From the deposit we get both outcome shares, so in a sense we
    // "bought" more. Also, we don't want to double-count this deposit as shares
    // "sold". So adjust for deposit to get an "effective" amount bought/sold.
    const deposit = Math.max(amountSell - userBalance[trade.assetSell], 0.0);
    const effectiveBuy = amountBuy + deposit;
    const effectiveSell = amountSell - deposit;
    const costPointsBuy = effectiveBuy * sharePricePoints;
    const costPointsSell = effectiveSell * (1.0 - sharePricePoints);

    maximizeExpectedLogWealth(p);

    var sellRow = effectiveSell === 0.0 ? "" : `
    <tr>
        <td>Sell</td>
        <td class="num amount">$\u{200a}${costPointsSell.toFixed(2)}</td>
        <td class="at">worth of</td>
        <td class="outcome-label">${labelSell},</td>
        <td class="num amount">${effectiveSell.toFixed(2)}</td>
        <td class="at">shares at</td>
        <td class="num price">$\u{200a}${(1.0 - sharePricePoints).toFixed(2)} per share</td>
    </tr>
    `;

    var depositRow = deposit === 0.0 ? "" : `
    <tr>
      <td>Lock</td>
      <td class="num amount">$\u{200a}${deposit.toFixed(2)}</td>
      <td colspan="5"> up in this market until resolution</td>
    </tr>
    `;

    offerTable.innerHTML = `
    <tr>
        <td>Buy</td>
        <td class="num amount strong">$\u{200a}${costPointsBuy.toFixed(2)}</td>
        <td class="at">worth of</td>
        <td class="outcome-label"><strong>${labelBuy}</strong>,</td>
        <td class="num amount">${effectiveBuy.toFixed(2)}</td>
        <td class="at">shares at</td>
        <td class="num price"><strong>$\u{200a}${sharePricePoints.toFixed(2)}</strong> per share</td>
    </tr>
    ${sellRow}
    ${depositRow}
    `;

    if (trade.valid) {
        document.trade_form.asset_in.value = assetIds[trade.assetSell];
        document.trade_form.asset_out.value = assetIds[trade.assetBuy];
        document.trade_form.amount_in.value = trade.amountSell.toFixed(2);
        // Build in 2% slippage tolerance.
        // TODO: Let the user pick.
        document.trade_form.min_out.value = (trade.amountBuy * 0.98).toFixed(6);
    }

    const submitButton = document.getElementById("trade-submit");
    submitButton.disabled = !trade.valid;
}

function initializeSlider() {
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
        getTrade(selectedProbability);
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

    if (canTrade) {
        knob.addEventListener("mousedown", onDragStart);
        knob.addEventListener("touchstart", onDragStart);
    }

    const p = getProbability(systemBalance);
    setPercentage(tMarket, pMarket, p);
    setPercentage(tUser, pUser, p);
    if (isOpen) {
        positionSlider(p);
    } else {
        knob.parentElement.removeChild(knob);
    }

    window.addEventListener("resize", onResize);
}

function initialize() {
    initializeSlider();
}

document.addEventListener("DOMContentLoaded", initialize);
