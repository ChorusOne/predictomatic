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
const max0 = systemBalance[0] + userBalance[0];
const pMin = Math.exp(-max0 / marketB) / invariant;

// Same for outcome 1, and for a binary question, that gives us the maximum
// probability we can afford to buy.
const max1 = systemBalance[1] + userBalance[1];
const pMax = 1.0 - Math.exp(-max1 / marketB) / invariant;

const canTrade = pMax - pMin > 0.001;

function getProbability(balance) {
    const ps = balance.map(lk => Math.exp(-lk / marketB));
    return ps[0] / invariant;
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
    const costPoints = trade.valid ? (1.0 / (ratio + 1.0)) : p;

    // Usually it's sufficient to quote the price in cents, you're not going
    // to care about the difference between a 67% and 68% probability. But
    // at the extreme prices, it may start to matter, so add a decimal there.
    const costPrecision = costPoints > 0.90 || costPoints < 0.10 ? 3 : 2;

    offerTable.innerHTML = `
    <tr>
        <td>You receive</td>
        <td class="num amount strong">${amountBuy.toFixed(2)}</td>
        <td class="outcome-label strong">${labelBuy}</td>
        <td class="at">at</td>
        <td class="num price"><strong>$\u{200a}${costPoints.toFixed(costPrecision)}</strong> per share</td>
    </tr>
    <tr>
        <td>You pay</td>
        <td class="num amount">${amountSell.toFixed(2)}</td>
        <td class="outcome-label">${labelSell}</td>
        <td class="at">at</td>
        <td class="num price">$\u{200a}${(1.0 - costPoints).toFixed(costPrecision)} per share</td>
    </tr>
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

function initializeDepositForm() {
    const input = document.getElementById("input-deposit-amount");
    const button = document.getElementById("button-deposit");
    input.addEventListener("input", (event) => {
        const n = Number(input.value.replace("$", ""));
        const isValid = Number.isFinite(n) && n > 0.0 && n <= userLiquidPoints;
        button.disabled = !isValid;
    });
}

function initialize() {
    initializeSlider();
    initializeDepositForm();
}

document.addEventListener("DOMContentLoaded", initialize);
