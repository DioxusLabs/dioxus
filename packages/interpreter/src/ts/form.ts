// Consistently deserialize forms and form elements for use across web/desktop/mobile

type FormValues = { [key: string]: FormDataEntryValue[] };

export function retriveValues(event: Event, target: HTMLElement): FormValues {
  const contents: FormValues = {};

  if (target instanceof HTMLFormElement && (event.type === "submit" || event.type === "input")) {
    retrieveFormValues(target, contents);
  }

  if (target instanceof HTMLSelectElement && (event.type === "input" || event.type === "change")) {
    retriveInputsValues(target, contents);
  }

  return contents;
}

export function retrieveFormValues(form: HTMLFormElement, contents: FormValues) {
  const formData = new FormData(form);

  for (let name in formData.keys()) {
    let element = form.elements.namedItem(name);

    // todo: this is going to be a problem for select-multiple?
    if (!(element instanceof HTMLInputElement)) {
      continue;
    }

    switch (element.type) {
      case "select-multiple":
        contents[name] = formData.getAll(name);
        break;

      // By default, it's just a single value
      default:
        contents[name] = [formData.get(name)];
        break;
    }
  }
}

export function retriveInputsValues(target: HTMLSelectElement, contents: FormValues,) {
  const selectData = target.options;
  contents["options"] = [];

  for (let i = 0; i < selectData.length; i++) {
    let option = selectData[i];
    if (option.selected) {
      contents["options"].push(option.value.toString());
    }
  }
}
