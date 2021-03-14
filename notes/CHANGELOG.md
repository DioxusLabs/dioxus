


# Project: Live-View 🤲 🍨
> Combine the server and client into a single file :) 


# Project: Sanitization (TBD)
> Improve code health
- [ ] (Macro) Clippy sanity for html macro
- [ ] (Macro) Error sanitization

# Project: Examples
> Get *all* the examples
- [ ] (Examples) Tide example with templating

# Project: State management 
> Get some global state management installed with the hooks API


# Project: Concurrency (TBD)
> Ensure the concurrency model works well, play with lifetimes to check if it can be multithreaded + halted


# Project: Web-View 🤲 🍨
> Proof of concept: stream render edits from server to client
- [x] Prove that the diffing and patching framework can support patch streaming

# Project: Web_sys renderer (TBD)
- [x] WebSys edit interpreter
- [x] Event system using async channels
- [ ] Implement conversion of all event types into synthetic events
# Project: String Render (TBD)
> Implement a light-weight string renderer with basic caching 
- [x] (Macro) Make VText nodes automatically capture and format IE allow "Text is {blah}"
- [ ] (SSR) Implement stateful 3rd party string renderer

# Project: Hooks + Context + Subscriptions (TBD)
> Implement the foundations for state management
- [x] Implement context object
- [x] Implement use_state (rewrite to use the use_reducer api like rei)
- [x] Implement use_ref
- [x] Implement use_context (only the API, not the state management solution)
- [ ] Implement use_reducer (WIP)

# Project: QOL 
> Make it easier to write components
- [x] (Macro) Tweak event syntax to not be dependent on wasm32 target (just return regular closures which get boxed/alloced)
- [x] (Macro) Tweak component syntax to accept a new custom element 
- [ ] (Macro) Allow components to specify their props as function args

# Project: Initial VDOM support (TBD)
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

