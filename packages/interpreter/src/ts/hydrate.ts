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
  // Get the template that marks the start of the placeholder we are replacing
  const placeholder_id = `ds-${suspense_placeholder_id}`;
  const startPlaceholder = document.getElementById(placeholder_id);
  // Get the node we are replacing it with
  const target = document.getElementById(`ds-${suspense_placeholder_id + 1}`);
  console.log(
    `swapping id ${suspense_placeholder_id} with id ${
      suspense_placeholder_id + 1
    }`
  );
  // Replace the placeholder with the children of the resolved div
  // First delete all nodes between the template and the comment <!--ds-{id}--> that marks the end of the placeholder

  let current = startPlaceholder.nextSibling;
  const endNode = (node: Node): boolean => {
    return (
      node.nodeType === Node.COMMENT_NODE && node.textContent === placeholder_id
    );
  };
  while (current && !endNode(current)) {
    const next = current.nextSibling;
    current.remove();
    current = next;
  }

  // Then replace the template with the children of the resolved div
  startPlaceholder.replaceWith(...target.childNodes);
  target.remove();
}
