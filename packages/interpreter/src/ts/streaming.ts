import "./hydrate_types";

window.dx_swap = (suspense_placeholder_id: number) => {
  // Get the placeholder node we are replacing
  const template = document.getElementById(`ds-${suspense_placeholder_id}`);
  // Get the node we are replacing it with
  const target = document.getElementById(`ds-${suspense_placeholder_id + 1}`);
  target.hidden = false;
  // Replace the placeholder with the resolved node
  template.replaceWith(target);
};
