import shim from "../build/worker/shim.mjs";

async function fetch(request, env, ctx) {
    Error.stackTraceLimit = 100;
    return shim.fetch(request, env, ctx);
}

export default { fetch };