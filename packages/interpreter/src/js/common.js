function setAttributeInner(node, field, value, ns) {
  if (ns === "style") {
    node.style.setProperty(field, value);
    return;
  }
  if (ns) {
    node.setAttributeNS(ns, field, value);
    return;
  }
  switch (field) {
    case "value":
      if (node.value !== value) node.value = value;
      break;
    case "initial_value":
      node.defaultValue = value;
      break;
    case "checked":
      node.checked = truthy(value);
      break;
    case "initial_checked":
      node.defaultChecked = truthy(value);
      break;
    case "selected":
      node.selected = truthy(value);
      break;
    case "initial_selected":
      node.defaultSelected = truthy(value);
      break;
    case "dangerous_inner_html":
      node.innerHTML = value;
      break;
    default:
      if (!truthy(value) && isBoolAttr(field)) node.removeAttribute(field);
      else node.setAttribute(field, value);
  }
}
function truthy(val) {
  return val === "true" || val === !0;
}
function isBoolAttr(field) {
  switch (field) {
    case "allowfullscreen":
    case "allowpaymentrequest":
    case "async":
    case "autofocus":
    case "autoplay":
    case "checked":
    case "controls":
    case "default":
    case "defer":
    case "disabled":
    case "formnovalidate":
    case "hidden":
    case "ismap":
    case "itemscope":
    case "loop":
    case "multiple":
    case "muted":
    case "nomodule":
    case "novalidate":
    case "open":
    case "playsinline":
    case "readonly":
    case "required":
    case "reversed":
    case "selected":
    case "truespeed":
    case "webkitdirectory":
      return !0;
    default:
      return !1;
  }
}
function retrieveFormValues(form) {
  let formData = new FormData(form),
    contents = {};
  return (
    formData.forEach((value, key) => {
      if (contents[key]) contents[key].push(value);
      else contents[key] = [value];
    }),
    { valid: form.checkValidity(), values: contents }
  );
}
export { setAttributeInner, retrieveFormValues };
