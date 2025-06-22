export function greeting(from, to) {
    return `Hello ${to}, this is ${from} speaking from JavaScript!`;
}

export function add(a, b) {
    return a + b;
}

export function processData(data) {
    return data.map(item => item.toUpperCase());
}