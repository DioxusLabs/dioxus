
export function get_form_data(form) {
    let values = new Map();
    const formData = new FormData(form);

    for (let name of formData.keys()) {
        const fieldType = target.elements[name].type;

        switch (fieldType) {
            case "select-multiple":
                contents.values[name] = formData.getAll(name);
                break;

            // add cases for fieldTypes that can hold multiple values here
            default:
                contents.values[name] = formData.get(name);
                break;
        }
    }

    return values;
}
