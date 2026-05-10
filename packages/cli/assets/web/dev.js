const STORAGE_KEY = "SCHEDULED-DX-TOAST";
let currentTimeout = null;
let currentToastId = 0;

// Show a toast, removing the previous one.
function showDXToast(headerText, message, progressLevel, durationMs) {
    const decor = document.getElementById("__dx-toast-decor");
    const text = document.getElementById("__dx-toast-text");
    const msg = document.getElementById("__dx-toast-msg");
    const inner = document.getElementById("__dx-toast-inner");
    const toast = document.getElementById("__dx-toast");

    if (decor) decor.className = `dx-toast-level-bar ${progressLevel}`;
    if (text) text.innerText = headerText;
    if (msg) msg.innerText = message;
    if (inner) inner.style.right = "0";
    if (toast) {
        toast.removeAttribute("aria-hidden");
        toast.addEventListener("click", closeDXToast);
    }

    // Wait a bit of time so animation plays correctly.
    setTimeout(
        () => {
            let ourToastId = currentToastId;
            currentTimeout = setTimeout(() => {
                if (ourToastId == currentToastId) {
                    closeDXToast();
                }
            }, durationMs);
        },
        100
    );

    currentToastId += 1;
}

// Schedule a toast to be displayed after reload.
function scheduleDXToast(headerText, message, level, durationMs) {
    let data = {
        headerText,
        message,
        level,
        durationMs,
    };

    let jsonData = JSON.stringify(data);
    sessionStorage.setItem(STORAGE_KEY, jsonData);
}

// Close the current toast.
function closeDXToast() {
    document.getElementById("__dx-toast-inner").style.right = "-1000px";
    document.getElementById("__dx-toast").setAttribute("aria-hidden", "true");
    clearTimeout(currentTimeout);
}

// Handle any scheduled toasts after reload.
let potentialData = sessionStorage.getItem(STORAGE_KEY);
if (potentialData) {
    sessionStorage.removeItem(STORAGE_KEY);
    let data = JSON.parse(potentialData);
    showDXToast(data.headerText, data.message, data.level, data.durationMs);
}

window.scheduleDXToast = scheduleDXToast;
window.showDXToast = showDXToast;
window.closeDXToast = closeDXToast;