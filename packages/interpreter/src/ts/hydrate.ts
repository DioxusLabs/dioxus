// Consistently deserialize forms and form elements for use across web/desktop/mobile
const id_to_string = (id: number): string => `ds-${id}`;
// @ts-ignore
window.dx_hydrate = (suspense_placeholder_id: number) => {
  // Get the placeholder node we are replacing
  const template = document.getElementById(
    id_to_string(suspense_placeholder_id)
  );
  // Get the node we are replacing it with
  const target = document.getElementById(
    id_to_string(suspense_placeholder_id + 1)
  );
  target.hidden = false;
  // Replace the placeholder with the resolved node
  template.replaceWith(target);
};
