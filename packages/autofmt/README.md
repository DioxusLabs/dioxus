# This crate autofmts blocks of rsx!

This crate formats rsx! by parsing call bodies and pretty-printing them back out.



# Todo:
Sorted roughly in order of what's possible

- [ ] Oneline rsx! calls - blocker because this wrecks formatting
- [ ] Nested RSX calls (important) - unnecessary but desirable
- [ ] RSX edits overstepping each other
- [ ] Collapse components and elements under syntax -
- [ ] Don't eat comments in exprs
- [ ] Format regular exprs
- [ ] Fix prettyplease around chaining
- [ ] Don't eat comments in prettyplease
