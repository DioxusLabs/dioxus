import "./hydrate_types";

// Chunks may load before the wasm has loaded and the callback to hydrate the dom is registered. We queue up hydration ids and data until the wasm is ready
window.hydrate_queue = [];

// @ts-ignore
window.dx_hydrate = (id: number, data: Uint8Array) => {
  if (window.hydration_callback) {
    window.hydration_callback(id, data);
  } else {
    window.hydrate_queue.push([id, data]);
  }
};
