(function (root, factory) {
    if (typeof exports === "object" && typeof module === "object") {
        root.__umd_minify_value = (module.exports = factory()).value;
    } else if (typeof define === "function" && define.amd) {
        define(function () {
            root.__umd_minify_value = factory().value;
            return factory();
        });
    } else {
        root.__umd_minify_value = factory().value;
    }
})(this, function () {
    return { value: "ok-umd" };
});
