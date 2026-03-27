// This script runs in a separate Web Worker thread.
// It receives a number via postMessage, computes the nth Fibonacci number,
// and posts the result back to the main thread.
//
// Because this runs in a worker, heavy computation does not block the UI.

self.onmessage = function (e) {
    const n = e.data;
    self.postMessage({ input: n, result: fib(n) });
};

function fib(n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}
