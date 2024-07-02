export {};

declare global {
  interface Window {
    hydrate_queue: [number[], Uint8Array][];
    hydration_callback: null | ((id: number[], data: Uint8Array) => void);
  }
}
