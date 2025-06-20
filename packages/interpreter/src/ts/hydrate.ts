import "./hydrate_types";
import { HydrationCallback } from "./hydrate_types";

export function register_rehydrate_chunk_for_streaming(callback: HydrationCallback): void {
  return register_rehydrate_chunk_for_streaming_debug(callback);
}

export function register_rehydrate_chunk_for_streaming_debug(
  callback: HydrationCallback
): void {
  window.hydration_callback = callback;
  for (let i = 0; i < window.hydrate_queue.length; i++) {
    const [id, data, debug_types, debug_locations] = window.hydrate_queue[i];
    window.hydration_callback(id, data, debug_types, debug_locations);
  }
}
