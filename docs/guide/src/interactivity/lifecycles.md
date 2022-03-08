# Effects

In theory, your UI code should be side-effect free. Whenever a component renders, all of its state should be prepared ahead of time. In reality, we often need to perform some sort of side effect. Possible effects include:

- Logging some data
- Pre-fetching some data
- Attaching code to native elements
- Cleaning up

This section is organized under interactivity because effects can be important to add things like transitions, videos, and other important media.

