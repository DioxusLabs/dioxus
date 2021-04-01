# Dioxus v0.1.0
Welcome to the first iteration of the Dioxus Virtual DOM! This release brings support for:
- Web via WASM
- Desktop via webview integration
- Server-rendering with custom Display Impl
- Liveview (experimental)
- Mobile (experimental)
- State management
- Build CLI
----
## Project: Initial VDOM support (TBD)
> Get the initial VDom + Event System + Patching + Diffing + Component framework up and running
> Get a demo working using just the web
- [x] (Core) Migrate virtual node into new VNode type
- [x] (Core) Arena allocate VNodes
- [x] (Core) Allow VNodes to borrow arena contents
- [x] (Core) Introduce the VDOM and patch API for 3rd party renderers
- [x] (Core) Implement lifecycle
- [x] (Core) Implement an event system 
- [x] (Core) Implement child nodes, scope creation
- [ ] (Core) Implement dirty tagging and compression

## Project: QOL 
> Make it easier to write components
- [x] (Macro) Tweak event syntax to not be dependent on wasm32 target (just return regular closures which get boxed/alloced)
- [x] (Macro) Tweak component syntax to accept a new custom element 
- [ ] (Macro) Allow components to specify their props as function args  (not going to do)

## Project: Hooks + Context + Subscriptions (TBD)
> Implement the foundations for state management
- [x] Implement context object
- [x] Implement use_state (rewrite to use the use_reducer api like rei)
- [x] Implement use_ref
- [x] Implement use_context (only the API, not the state management solution)
- [ ] Implement use_reducer (WIP)

## Project: String Render (TBD)
> Implement a light-weight string renderer with basic caching 
- [x] (Macro) Make VText nodes automatically capture and format IE allow "Text is {blah}"
- [x] (SSR) Implement stateful 3rd party string renderer

## Project: Web_sys renderer (TBD)
- [x] WebSys edit interpreter
- [x] Event system using async channels
- [ ] Implement conversion of all event types into synthetic events

## Project: Web-View 🤲 🍨
> Proof of concept: stream render edits from server to client
- [x] Prove that the diffing and patching framework can support patch streaming

## Project: Examples
> Get *all* the examples
- [ ] (Examples) Tide example with templating

## Project: State management 
> Get some global state management installed with the hooks + context API


## Project: Concurrency (TBD)
> Ensure the concurrency model works well, play with lifetimes to check if it can be multithreaded + halted
?


## Project: Mobile exploration


## Project: Live-View 🤲 🍨
> Combine the server and client into a single file :) 


## Project: Sanitization (TBD)
> Improve code health
- [ ] (Macro) Clippy sanity for html macro
- [ ] (Macro) Error sanitization


## Outstanding todos:
> anything missed so far
- [ ] dirty tagging, compression
- [ ] fragments
- [ ] make ssr follow HTML spec
- [ ] code health
- [ ] miri tests
- [ ] todo mvc
- [ ] fix
- [ ] node refs (postpone for future release?)
- [ ] styling built-in (future release?)
- [ ] key handler?
- [ ] FC macro
- [ ] Documentation overhaul
- [ ] Website
- [x] keys on components
- [ ] fix keys on elements
- [ ] all synthetic events filed out
- [ ] doublecheck event targets and stuff
