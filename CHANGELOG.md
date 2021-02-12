# Project: Live-View 🤲 🍨
> Combine the server and client into a single file :) 


# Project: Sanitization (TBD)
- [ ] (Macro) Clippy sanity for html macro
- [ ] (Macro) Error sanitization

# Project: Examples
> Get *all* the examples
- [ ] (Examples) Tide example with templating

# Project: State management 
> Get some global state management installed with the hooks API


# Project: Concurrency (TBD)
> Ensure the concurrency model works well, play with lifetimes to check if it can be multithreaded + halted

# Project: Web_sys renderer (TBD)
- [ ] (Web) Web-sys renderer and web tests

# Project: String Render (TBD)
> Implement a light-weight string renderer with basic caching 
- [ ] (SSR) Implement stateful 3rd party string renderer
- [ ] (Macro) Make VText nodes automatically capture and format IE allow "Text is {blah}" in place of {format!("Text is {}",blah)}

# Project: Hooks + Context + Subscriptions (TBD)
> Implement the foundations for state management
- [x] Implement context object
- [x] Implement use_state
- [x] Implement use_ref
- [ ] Implement use_reducer
- [ ] Implement use_context

# Project: QOL 
> Make it easier to write components
- [ ] (Macro) Tweak event syntax to not be dependent on wasm32 target (just return regular closures which get boxed)
- [ ] (Macro) Tweak component syntax to accept a new custom element 
- [ ] (Macro) Allow components to specify their props as function args

# Project: Initial VDOM support (TBD)
> Get the initial VDom + Event System + Patching + Diffing + Component framework up and running
> Get a demo working using just the web
- [x] (Core) Migrate virtual node into new VNode type
- [x] (Core) Arena allocate VNodes
- [x] (Core) Allow VNodes to borrow arena contents
- [x] (Core) Introduce the VDOM and patch API for 3rd party renderers
- [ ] (Core) Implement lifecycle
- [ ] (Core) Implement an event system 

