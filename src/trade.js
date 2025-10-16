function getProbability(balance) {
    const ps = balance.map(lk => Math.exp(-lk));
    const sum = ps.reduce((acc, x) => acc + x, 0.0);
    // For binary questions, outcome 0 is the one we are interested in.
    return ps[0] / sum;
}

// Return the delta in pool shares to bring the implied probability to `p`.
function getTrade(p) {
    // Comput the current value of the invariant. Whatever we swap, the
    // invariant must remain the same.
    const ps = systemBalance.map(lk => Math.exp(-lk));
    const invariant = ps.reduce((acc, x) => acc + x, 0.0);

    // const targetDiff = Math.log(p / (1.0 - p));
    // const balanceYesNew = -Math.log(invariant / (1 + Math.exp(-targetDiff)));
    // const balanceNew = [balanceYesNew, balanceYesNew + targetDiff];

    const newYes = -Math.log(p * invariant);
    const newNo = -Math.log(invariant - Math.exp(-newYes));
    // Equivalent:
    const newNo2 = -Math.log(invariant - p * invariant);
    // Equivalent:
    const newNo3 = -Math.log(invariant * (1.0 - p));

    const p2 = getProbability([newYes, newNo3]);

    console.log(p, newYes, newNo, newNo2, newNo3, p2);
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
        const rxClamp = Math.max(0.001, Math.min(0.999, rx));
        positionSlider(rxClamp);
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
