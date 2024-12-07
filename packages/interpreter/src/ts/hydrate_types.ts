export {};

declare global {
  interface Window {
    hydrate_queue: [number[], Uint8Array, string[] | null, string[] | null][];
    hydration_callback:
      | null
      | ((
          id: number[],
          data: Uint8Array,
          debug_types: string[] | null,
          debug_locations: string[] | null
        ) => void);
  }
}
