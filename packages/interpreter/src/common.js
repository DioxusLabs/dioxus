export function setAttributeInner(node, field, value, ns) {
  const name = field;
  if (ns === "style") {
    // ????? why do we need to do this
    if (node.style === undefined) {
      node.style = {};
    }
    node.style[name] = value;
  } else if (ns != null && ns != undefined) {
    node.setAttributeNS(ns, name, value);
  } else {
    switch (name) {
      case "value":
        if (value !== node.value) {
          node.value = value;
        }
        break;
      case "initial_value":
        node.defaultValue = value;
        break;
      case "checked":
        node.checked = value === "true" || value === true;
        break;
      case "selected":
        node.selected = value === "true" || value === true;
        break;
      case "dangerous_inner_html":
        node.innerHTML = value;
        break;
      default:
        // https://github.com/facebook/react/blob/8b88ac2592c5f555f315f9440cbb665dd1e7457a/packages/react-dom/src/shared/DOMProperty.js#L352-L364
        if (value === "false" && bool_attrs.hasOwnProperty(name)) {
          node.removeAttribute(name);
        } else {
          node.setAttribute(name, value);
        }
    }
  }
}