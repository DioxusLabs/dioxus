import "./hydrate_types";

export function register_rehydrate_chunk_for_streaming(
  callback: (id: number[], data: Uint8Array) => void
): void {
  window.hydration_callback = callback;
  for (let i = 0; i < window.hydrate_queue.length; i++) {
    const [id, data] = window.hydrate_queue[i];
    window.hydration_callback(id, data);
  }
}
