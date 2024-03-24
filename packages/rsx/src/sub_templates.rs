//! Logic for handlng sub-templates
//!
//! These are things like for/if and component calls that are expanded by the contaning macro but turn into templates
//! separate template calls in the final output.
//!
//! Instead of recursively expanding sub-templates in place - and thus losing location information - we collect them
//! into a list and then expand them at the end.
//!
//! The goal here is to provide a stable index for each sub-template in a way that we can find the for/if statements later.
//! This requires some sort of hashing + indexing of the sub-templates.
//!
//! When expanding the for/if statements, we still want to expand them in the correct order, but we defer the actual token
//! expansion until the end. This way the `TEMPLATE` static that subtemplate references is available *before* we convert
//! an `if` statement into an actual vnode.
//!
//! Expanding a template will generate two token streams - one for the vnode creation and one for list of templates.
//!
//! While semantically it might make sense for a macro call that sees its dynamic contents to own those static nodes,
//! core is currently designed around the assumption that the list of Dynamic attributes matches 1:1 the template it
//! belongs to. This means that whatever dynamic nodes are rendered need to match the template they are rendered in,
//! hence why we need to spit out true subtemplates rather than just building a "super template".
//!
//! I would prefer we used the "super template" strategy since it's more flexible when diffing - no need to keep track
//! of nested templates, and we can just diff the super template and the sub templates as a whole.
//!
//! However core would need to be redesigned to support this strategy, and I'm not sure if it's worth the effort.
//! It's just conceptually easier to have the sub-templates be a part of the template they are rendered in, and then
//! deal with the complicated rsx diffing logic in the rsx crate itself.
//!
//! /// The subtemplates that we've discovered that need to be updated
//! /// These will be collected just by recursing the body nodes of various items like for/if/components etc
//! ///
//! /// The locations of these templates will be the same location as the original template, with
//! /// the addition of the dynamic node index that the template belongs to.
//! ///
//! /// rsx! {                        <-------- this is the location of the template "file.rs:123:0:0"
//! ///     div {
//! ///         for i in 0..10 {      <-------- dyn_node(0)
//! ///             "hi"              <-------- template with location  "file.rs:123:0:1" (original name, but with the dynamic node index)
//! ///         }
//! ///     }
//! /// }
//! ///
//! /// We only track this when the template changes
//! pub discovered_templates: Vec<Template>,
