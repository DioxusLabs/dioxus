class ResizeEventDetail {
  readonly borderBoxSize: ResizeObserverSize;
  readonly contentBoxSize: ResizeObserverSize;
  readonly contentRect: DOMRectReadOnly
  readonly target: Element;

  constructor(
    borderBoxSize: ResizeObserverSize,
    contentBoxSize: ResizeObserverSize,
    contentRect: DOMRectReadOnly,
    target: Element,
  ) {
    this.borderBoxSize = borderBoxSize;
    this.contentBoxSize = contentBoxSize;
    this.contentRect = contentRect;
    this.target = target;
  }
}

export {
  ResizeEventDetail
};
