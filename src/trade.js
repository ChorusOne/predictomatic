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

// Return the delta in pool shares to bring the implied probability to `p`.
function getTrade(p) {
    const newYes = -Math.log(invariant * p) * marketB;
    const newNo = -Math.log(invariant * (1.0 - p)) * marketB;
    const dYes = newYes - systemBalance[0];
    const dNo = newNo - systemBalance[0];
    const offerElem = document.getElementById("trade-offer");

    // TODO: Don't hard-code the names here.
    if (dYes > 0.005) {
        const buyNo = (-dNo).toFixed(2);
        const sellYes = dYes.toFixed(2);
        offerElem.innerText = `Trade offer: ${buyNo} No for ${sellYes} Yes.`;
    } else if (dNo > 0.005) {
        const buyYes = (-dYes).toFixed(2);
        const sellNo = dNo.toFixed(2);
        offerElem.innerText = `Trade offer: ${buyYes} Yes for ${sellNo} No.`;
    } else if (canTrade) {
        offerElem.innerText = "Move the slider to receive a trade offer.";
    } else {
        offerElem.innerText = "To participate in this market, first deposit some funds on the right.";
    }

    console.log(p, newYes, newNo, getProbability([newYes, newNo]));
}

function initializeSlider() {
    const widget = document.getElementById("trade-widget");
    const knob = widget.getElementsByClassName("knob")[0];
    const hr = widget.getElementsByTagName("hr")[0];
    const tMarket = widget.getElementsByClassName("tmarket")[0];
    const pMarket = widget.getElementsByClassName("pmarket")[0];
    const tUser = widget.getElementsByClassName("tuser")[0];
    const pUser = widget.getElementsByClassName("puser")[0];

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
    positionSlider(p);
    setPercentage(tMarket, pMarket, p);
    setPercentage(tUser, pUser, p);
}

function initialize() {
    initializeSlider();
}

document.addEventListener("DOMContentLoaded", initialize);
