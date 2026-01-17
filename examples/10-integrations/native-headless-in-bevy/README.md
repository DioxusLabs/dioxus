Example: Integrating Dioxus into a Bevy application

This example demonstrates rendering a Dioxus application onto a texture within a Bevy application.

## Core Concepts
- Render a headless Dioxus-native app to a texture.
- Share and display the texture on a quad in the Bevy app.
- Transmit mouse and keyboard events from Bevy to Dioxus.
  - Events are captured when hovering over elements with the `catch-events` class, ensuring only Dioxus receives them.
- Manage application state through channel messages between Dioxus and Bevy.

