requestIdleCallback and requestAnimationFrame implementation

These currently actually slow down our DOM patching and thus are temporarily removed. Technically we can schedule around rIC and rAF but choose not to.
