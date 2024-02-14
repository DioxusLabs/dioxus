import shim, { getMemory, wasmModule } from "./worker/shim.mjs"

async function fetch(request, env, ctx) {
    Error.stackTraceLimit = 100;

    const asset = await env.ASSETS.fetch(request.clone());
    if (asset.ok) {
        return asset;
    } else {
        try {
            return shim.fetch(request, env, ctx);
        } catch (err) {
            const memory = getMemory();
            const coredumpService = env.COREDUMP_SERVICE;
            await recordCoredump({memory, wasmModule, request, coredumpService});
            throw err;
        }
    }
}

export default { fetch };