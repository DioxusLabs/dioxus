class ResizeEventDetail {
  readonly borderBoxSize: ResizeObserverSize;
  readonly contentBoxSize: ResizeObserverSize;
  readonly contentRect: DOMRectReadOnly;
  readonly target: Element;

  constructor(entry: ResizeObserverEntry) {
    this.borderBoxSize = entry.borderBoxSize?.[0];
    this.contentBoxSize = entry.contentBoxSize?.[0];
    this.contentRect = entry.contentRect;
    this.target = entry.target;
  }
}

export { ResizeEventDetail };
