import "./hydrate_types";

export function register_rehydrate_chunk_for_streaming(
  callback: (id: number[], data: Uint8Array) => void
): void {
  window.hydration_callback = (id: number[], data: Uint8Array) => {
    callback(id, data);
  };
  for (let i = 0; i < window.hydrate_queue.length; i++) {
    const [id, data] = window.hydrate_queue[i];
    window.hydration_callback(id, data);
  }
}

export function dx_swap(suspense_placeholder_id: number[]) {
  // Get the template that marks the start of the placeholder we are replacing
  const comma_separated_id = suspense_placeholder_id.join(",");
  const placeholder_id = `ds-${comma_separated_id}`;
  const startPlaceholder = document.getElementById(placeholder_id);
  // Get the node we are replacing it with
  const target = document.getElementById(`ds-${comma_separated_id}-r`);
  console.log(
    `swapping id ${suspense_placeholder_id} with id ${comma_separated_id}-r`
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
