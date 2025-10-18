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
    if (d0 > 0.005) {
        trade = {
            assetBuy: 1,
            assetSell: 0,
            amountBuy: -d1,
            amountSell: d0,
        };
    } else if (d1 > 0.005) {
        trade = {
            assetBuy: 0,
            assetSell: 1,
            amountBuy: -d0,
            amountSell: d1,
        };
    }

    const offerElem = document.getElementById("trade-offer");

    if (trade !== null) {
        const amountBuy = trade.amountBuy;
        const amountSell = trade.amountSell;
        const labelBuy = assetLabels[trade.assetBuy];
        const labelSell = assetLabels[trade.assetSell];
        const ratio = trade.amountBuy / trade.amountSell;
        const costPoints = (1.0 / (ratio + 1.0));

        // Usually it's sufficient to quote the price in cents, you're not going
        // to care about the difference between a 67% and 68% probability. But
        // at the extreme prices, it may start to matter, so add a decimal there.
        const costPrecision = costPoints > 0.90 || costPoints < 0.10 ? 3 : 2;

        offerElem.innerHTML = `
        <h3>Trade offer</h3>
        <table>
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
        </table>
        `;

        document.trade_form.asset_in.value = assetIds[trade.assetSell];
        document.trade_form.asset_out.value = assetIds[trade.assetBuy];
        document.trade_form.amount_in.value = trade.amountSell.toFixed(2);
        // Build in 2% slippage tolerance.
        // TODO: Let the user pick.
        document.trade_form.min_out.value = (trade.amountBuy * 0.98).toFixed(6);
    } else if (canTrade) {
        // TODO: This makes the page flicker when you slide past the zero point,
        // maybe it's better to at least display the regular trade offer table?
        offerElem.innerText = "Move the slider to receive a trade offer.";
    } else {
        offerElem.innerText = "To participate in this market, first deposit some funds on the right.";
    }

    const submitButton = document.getElementById("trade-submit");
    submitButton.disabled = trade === null;
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
        getTrade(p);
    };
    const onDragMove = (event) => {
        const dx = event.clientX - startX;
        const rx = (knobRect.left + 0.5 * knobRect.width - hrRect.left + dx) / hrRect.width;
        // Clamp to [0.1%, 99.9%], or what the user can afford, whichever is
        // narrower.
        const min = Math.max(pMin, 0.001);
        const max = Math.min(pMax, 0.999);
        const p = Math.max(min, Math.min(max, rx));
        positionSlider(p);
    };
    const onDragEnd = (event) => {
        document.removeEventListener("mouseup", onDragEnd);
        document.removeEventListener("touchend", onDragEnd);
        document.removeEventListener("mousemove", onDragMove);
        document.removeEventListener("touchmove", onDragMove);
    };
    const onDragStart = (event) => {
        widgetRect = widget.getClientRects()[0];
        knobRect = knob.getClientRects()[0];
        hrRect = hr.getClientRects()[0];
        startX = event.clientX;
        startY = event.clientY;
        document.addEventListener("mouseup", onDragEnd);
        document.addEventListener("touchend", onDragEnd);
        document.addEventListener("mousemove", onDragMove);
        document.addEventListener("touchmove", onDragMove);
    };

    knob.addEventListener("mousedown", onDragStart);
    knob.addEventListener("touchstart", onDragStart);

    const p = getProbability(systemBalance);
    setPercentage(tMarket, pMarket, p);
    setPercentage(tUser, pUser, p);
    if (isOpen) {
        positionSlider(p);
    } else {
        knob.parentElement.removeChild(knob);
    }
}

function initializeDepositForm() {
    const input = document.getElementById("input-deposit-amount");
    const button = document.getElementById("button-deposit");
    input.addEventListener("input", (event) => {
        const n = Number(input.value);
        const isValid = Number.isFinite(n) && n > 0.0 && n <= userLiquidPoints;
        button.disabled = !isValid;
    });
}

function initialize() {
    initializeSlider();
    initializeDepositForm();
}

document.addEventListener("DOMContentLoaded", initialize);
