# Notes on the web implementation

Here's some useful resources if you ever need to splunk into the intricacies of how events are handled in HTML:


- Not all event handlers are sync: https://w3c.github.io/uievents/#sync-async
- Some attributes are truthy: https://github.com/facebook/react/blob/8b88ac2592c5f555f315f9440cbb665dd1e7457a/packages/react-dom/src/shared/DOMProperty.js#L352-L364
