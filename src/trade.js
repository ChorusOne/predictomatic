function initialize() {
    const widget = document.getElementById("trade-widget");
    const knob = widget.getElementsByClassName("knob")[0];
    const hr = widget.getElementsByTagName("hr")[0];

    var startX = 0.0;
    var startY = 0.0;
    var widgetRect = widget.getClientRects()[0];
    var knobRect = knob.getClientRects()[0];
    var hrRect = hr.getClientRects()[0];

    const onDragMove = (event) => {
        const dx = event.clientX - startX;
        const dy = event.clientY - startY;
        const rx = (knobRect.left + 0.5 * knobRect.width - hrRect.left + dx) / hrRect.width;
        const rxClamp = Math.max(0.0, Math.min(1.0, rx));
        const xClamp = rxClamp * hrRect.width - knobRect.width * 0.5;
        const x = hrRect.left - widgetRect.left + xClamp;
        console.log("Dragging ...", dx, rxClamp);
        knob.style.left = `${x}px`;
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
}

document.addEventListener("DOMContentLoaded", initialize);
