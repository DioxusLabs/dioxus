import shim, { getMemory, wasmModule } from "../build/worker/shim.mjs"

async function fetch(request, env, ctx) {
    Error.stackTraceLimit = 100;
    try {
        return shim.fetch(request, env, ctx);
    } catch (err) {
        const memory = getMemory();
        const coredumpService = env.COREDUMP_SERVICE;
        await recordCoredump({ memory, wasmModule, request, coredumpService });
        throw err;
    }
}

export default { fetch };