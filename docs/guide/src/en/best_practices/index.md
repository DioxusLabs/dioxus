# Best Practices

## Reusable Components

As much as possible, break your code down into small, reusable components and hooks, instead of implementing large chunks of the UI in a single component. This will help you keep the code maintainable – it is much easier to e.g. add, remove or re-order parts of the UI if it is organized in components.

Organize your components in modules to keep the codebase easy to navigate!

## Minimize State Dependencies

While it is possible to share state between components, this should only be done when necessary. Any component that is associated with a particular state object needs to be re-rendered when that state changes. For this reason:

- Keep state local to a component if possible
- When sharing state through props, only pass down the specific data necessary
