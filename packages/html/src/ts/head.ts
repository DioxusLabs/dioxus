// Helper functions for working with the document head

export function createElementInHead(
  tag: string,
  attributes: [string, string][]
): void {
  const element = document.createElement(tag);
  for (const [key, value] of attributes) {
    element.setAttribute(key, value);
  }
  document.head.appendChild(element);
}
