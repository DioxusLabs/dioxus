const requestIdleCallback =
  (typeof self !== 'undefined' &&
    self.requestIdleCallback &&
    self.requestIdleCallback.bind(window)) ||
  function (cb) {
    const start = Date.now();
    return setTimeout(() => {
      cb({
        didTimeout: false,
        timeRemaining: function () {
          return Math.max(0, 50 - (Date.now() - start));
        },
      });
    }, 1);
  };

const cancelIdleCallback =
  (typeof self !== 'undefined' &&
    self.cancelIdleCallback &&
    self.cancelIdleCallback.bind(window)) ||
  function (id) {
    return clearTimeout(id);
  };

if (typeof window !== 'undefined') {
  window.requestIdleCallback = requestIdleCallback;
  window.cancelIdleCallback = cancelIdleCallback;
}
