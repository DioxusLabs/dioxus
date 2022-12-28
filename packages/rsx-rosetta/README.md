# Rosetta for RSX
---

Dioxus sports its own templating language inspired by C#/Kotlin/RTMP, etc. It's pretty straightforward.

However, it's NOT HTML. This is done since HTML is verbose and you'd need a dedicated LSP or IDE integration to get a good DX in .rs files.

RSX is simple... It's similar enough to regular Rust code to trick most IDEs into automatically providing support for things like block selections, folding, highlighting, etc.

To accomodate the transition from HTML to RSX, you might need to translate some existing code.

This library provids a central AST that can accept a number of inputs:

- HTML
- Syn (todo)
- Akama (todo)
- Jinja (todo)

From there, you can convert directly to a string or into some other AST.
