# This module includes all life-cycle related mechanics, including the virtual DOM, scopes, properties, and lifecycles.
---
The VirtualDom is designed as so:

VDOM contains:
- An arena of component scopes.
    - A scope contains
        - lifecycle data
        - hook data
- Event queue
    - An event

A VDOM is
- constructed from anything that implements "component"

A "Component" is anything (normally functions) that can be ran with a context to produce VNodes
- Must implement properties-builder trait which produces a properties builder

A Context
- Is a consumable struct
    - Made of references to properties
    - Holds a reference (lockable) to the underlying scope
    - Is partially thread-safe
