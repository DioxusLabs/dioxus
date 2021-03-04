# Safety

We don't claim to be a "safe" library. We want to be 100% safe, and will create tests and validate that we are, but our priorities for this library are:

- productivity
- performance
- safety
  
We are willing to use sharp tools (ie transmuting self-referential pointer types) in order to achieve higher ergonomics (ie returning self-referential listeners). 

However, we can only use these sharp tools if we verify that it's not possible to write user-facing code that breaks safety guarantees. For internal code... well, whoever contributes needs to understand the architecture and read the comments related to safety.

We are doing one of the more annoying things to do with Rust: self-referential graph structures. VNodes reference a bump arena which is contained by scope. Conveniently, the bump arenas also belong to scope, and now we have a self-referential struct. 

We */would/* use a solution like ourborous or self_referential, but these libraries generate "imaginary code" that doesn't integrate with RA. It's simpler and easier to review if we set some rules on what is/isn't allowed.

Here's the two main sources of unsafety:
- 1) vnodes, bump arenas, and scope
- 2) context_api and use_context
  
For 1), we can fairly confidently guarantee safety by being careful about lifetime casts and by using tools like refcell as flags. 

For 2), use_context authors (mostly state management) can implement either the Unsafe API or the Safe API. The Safe API is less performant, but will likely do everything you need. The Unsafe API is more performant, but will bite you if you don't implement it properly. Always validate with MIRI!

Because of 2), we provide two state management solutions (D-Reducer and D-Dataflow) that use the Unsafe API, but will still be 100% safe.

