/** 
 * This is a doc comment
 * second line
*/
export function greeting(from, to) {
    return `Hello ${to}, this is ${from} speaking from JavaScript!`;
}

/// This is another doc comment
export function add(a, b) {
    return a + b;
}

export function processData(data) {
    return data.map(item => item.toUpperCase());
}