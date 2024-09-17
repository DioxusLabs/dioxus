// Helper functions for working with the document head

function createElementInHead(
  tag: string,
  attributes: [string, string][],
  children: string | null
): void {
  const element = document.createElement(tag);
  for (const [key, value] of attributes) {
    element.setAttribute(key, value);
  }
  if (children) {
    element.appendChild(document.createTextNode(children));
  }
  document.head.appendChild(element);
}

// @ts-ignore
window.createElementInHead = createElementInHead;
