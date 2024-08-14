// Consistently deserialize forms and form elements for use across web/desktop/mobile

type FormValues = {
  valid?: boolean;
  values: { [key: string]: FormDataEntryValue[] };
}

export function retrieveValues(event: Event, target: HTMLElement): FormValues {
  let contents: FormValues = {
    values: {}
  };

  // If there's a form...
  let form = target.closest("form");

  // If the target is an input, and the event is input or change, we want to get the value without going through the form
  if (form) {
    if (
      event.type === "input"
      || event.type === "change"
      || event.type === "submit"
      || event.type === "reset"
      || event.type === "click"
    ) {
      contents = retrieveFormValues(form);
    }
  }

  return contents;
}

// todo: maybe encode spaces or something?
// We encode select multiple as a comma separated list which breaks... when there's commas in the values
export function retrieveFormValues(form: HTMLFormElement): FormValues {
  const formData = new FormData(form);
  const contents: { [key: string]: FormDataEntryValue[] } = {};
  formData.forEach((value, key) => {
    if (contents[key]) {
      contents[key].push(value);
    } else {
      contents[key] = [value];
    }
  });
  return {
    valid: form.checkValidity(),
    values: contents
  };
}

export function retrieveSelectValue(target: HTMLSelectElement): string[] {
  // there might be multiple...
  let options = target.selectedOptions;
  let values = [];
  for (let i = 0; i < options.length; i++) {
    values.push(options[i].value);
  }
  return values;
}
