import shim, { getMemory, wasmModule } from "./worker/shim.mjs"

async function handleSession(websocket) {
    websocket.accept()
    websocket.addEventListener("message", async message => {
        console.log(message)
    })

    websocket.addEventListener("close", async evt => {
        // Handle when a client closes the WebSocket connection
        console.log(evt)
    })
}

async function handleWebsocket(request, env, ctx) {
    const [client, server] = Object.values(new WebSocketPair());
    await handleSession(server);
    return new Response(null, { status: 101, webSocket: client });
}

async function fetch(request, env, ctx) {
    Error.stackTraceLimit = 100;

    // console.log(request.url);
    // if (request.headers.get("Upgrade") === "websocket") {
    //     return handleWebsocket(request, env, ctx);
    // }

    // const asset = await env.ASSETS.fetch(request.clone());
    // if (asset.ok) {
    //     return asset;
    // } else {
    //     try {
            return shim.fetch(request, env, ctx);
        // } catch (err) {
        //     const memory = getMemory();
        //     const coredumpService = env.COREDUMP_SERVICE;
        //     await recordCoredump({memory, wasmModule, request, coredumpService});
        //     throw err;
        // }
    // }
}

export default { fetch };