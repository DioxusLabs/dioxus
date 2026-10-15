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

  // Track when inserted scripts and stylesheets are ready. Desktop evals wait for this chain.
  const isScript = tag === "script" && element.hasAttribute("src");
  const isStylesheet =
    tag === "link" &&
    element.getAttribute("rel") === "stylesheet" &&
    element.hasAttribute("href") &&
    !(element as HTMLLinkElement).disabled;
  if (isScript || isStylesheet) {
    const ready = new Promise<void>((resolve) => {
      element.addEventListener("load", () => resolve(), { once: true });
      element.addEventListener("error", () => resolve(), { once: true });
    });
    // @ts-ignore
    window.__dioxusHeadReady = (window.__dioxusHeadReady || Promise.resolve()).then(
      () => ready
    );
  }
}

// @ts-ignore
window.createElementInHead = createElementInHead;