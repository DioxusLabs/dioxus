import "./hydrate_types";

// Chunks may load before the wasm has loaded and the callback to hydrate the dom is registered. We queue up hydration ids and data until the wasm is ready
window.hydrate_queue = [];

// @ts-ignore
window.dx_hydrate = (id: number[], data: string) => {
  // First convert the base64 encoded string to a Uint8Array
  const decoded = atob(data);
  const bytes = Uint8Array.from(decoded, (c) => c.charCodeAt(0));
  if (window.hydration_callback) {
    window.hydration_callback(id, bytes);
  } else {
    window.hydrate_queue.push([id, bytes]);
  }
};
