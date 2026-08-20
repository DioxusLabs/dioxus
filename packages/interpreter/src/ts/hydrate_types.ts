export { };

export type HydrationCallback = (
  id: number[],
  data: Uint8Array,
  debug_types?: string[] | null,
  debug_locations?: string[] | null
) => void;

export type HydrationChunk = [
  id: number[],
  data: Uint8Array,
  debug_types?: string[] | null,
  debug_locations?: string[] | null,
];

declare global {
  interface Window {
    hydrate_queue: HydrationChunk[];
    hydration_callback:
    | null
    | HydrationCallback;
  }
}
