import "./hydrate_types";

export function register_rehydrate_chunk_for_streaming(
  callback: (id: number, data: Uint8Array) => void
): void {
  window.hydration_callback = (id: number, data: Uint8Array) => {
    callback(id, data);
  };
  for (let i = 0; i < window.hydrate_queue.length; i++) {
    const [id, data] = window.hydrate_queue[i];
    window.hydration_callback(id, data);
  }
}

export function dx_swap(suspense_placeholder_id: number) {
  // Get the placeholder node we are replacing
  const template = document.getElementById(`ds-${suspense_placeholder_id}`);
  // Get the node we are replacing it with
  const target = document.getElementById(`ds-${suspense_placeholder_id + 1}`);
  console.log(
    `swapping id ${suspense_placeholder_id} with id ${
      suspense_placeholder_id + 1
    }`
  );
  // Replace the placeholder with the children of the resolved div
  template.replaceWith(...target.childNodes);
  target.remove();
}
