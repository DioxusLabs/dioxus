export { };

export type HydrationCallback = (
  id: number[],
  data: Uint8Array,
  debug_types: string[] | null,
  debug_locations: string[] | null
) => void;

declare global {
  interface Window {
    hydrate_queue: [number[], Uint8Array, string[] | null, string[] | null][];
    hydration_callback:
    | null
    | HydrationCallback;
  }
}
