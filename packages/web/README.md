# Dioxus-Web

Build interactive user experiences directly in the web browser!

Dioxus-web provides a `WebsysRenderer` for the Dioxus Virtual Dom that handles events, progresses components, and updates the actual DOM using web-sys methods.


## Web-specific Optimizations
- Uses string interning of all common node types
- Optimistically interns short strings
- Builds trees completely before mounting them
