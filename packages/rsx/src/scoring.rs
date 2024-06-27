use crate::{Attribute, AttributeValue, BodyNode, HotLiteralType};

/// Take two nodes and return their similarity score
///
/// This is not normalized or anything, so longer nodes will have higher scores
pub fn score_dynamic_node(old_node: &BodyNode, new_node: &BodyNode) -> usize {
    // If they're different enums, they are not the same node
    if std::mem::discriminant(old_node) != std::mem::discriminant(new_node) {
        return 0;
    }

    use BodyNode::*;

    match (old_node, new_node) {
        (Element(_), Element(_)) => unreachable!("Elements are not dynamic nodes"),

        (Text(left), Text(right)) => {
            // We shouldn't be seeing static text nodes here
            assert!(!left.input.is_static() && !right.input.is_static());
            left.input.hr_score(&right.input)
        }

        (RawExpr(a), RawExpr(b)) if a == b => usize::MAX,

        (Component(a), Component(b)) => {
            // First, they need to be the same name, generics, and fields - those can't be added on the fly
            if a.name != b.name || a.generics != b.generics || a.fields.len() != b.fields.len() {
                return 0;
            }

            // Now, the contents of the fields might've changed
            // That's okay... score each one
            // we don't actually descend into the children yet...
            // If you swapped two components and somehow their signatures are the same but their children are different,
            // it might cause an unnecessary rebuild
            let mut score = 1;

            let mut left_fields = a.fields.iter().collect::<Vec<_>>();
            left_fields.sort_by(|a, b| a.name.to_string().cmp(&b.name.to_string()));

            let mut right_fields = b.fields.iter().collect::<Vec<_>>();
            right_fields.sort_by(|a, b| a.name.to_string().cmp(&b.name.to_string()));

            for (left, right) in left_fields.iter().zip(right_fields.iter()) {
                let scored = match score_attribute(&left, &right) {
                    usize::MAX => 3,
                    a if a == usize::MAX - 1 => 2,
                    a => a,
                };

                if scored == 0 {
                    return 0;
                }

                score += scored;
            }

            score
        }

        (ForLoop(a), ForLoop(b)) => {
            if a.pat != b.pat || a.expr != b.expr {
                return 0;
            }

            // The bodies don't necessarily need to be the same, but we should throw some simple heuristics at them to
            // encourage proper selection
            let mut score = 1;

            if a.body.roots.len() == b.body.roots.len() {
                score += 1;
            }

            if a.body.node_paths.len() == b.body.node_paths.len() {
                score += 1;
            }

            if a.body.attr_paths.len() == b.body.attr_paths.len() {
                score += 1;
            }

            score
        }

        (IfChain(a), IfChain(b)) => {
            if a.cond != b.cond {
                return 0;
            }

            // The bodies don't necessarily need to be the same, but we should throw some simple heuristics at them to
            // encourage proper selection
            let mut score = 1;

            if a.then_branch.roots.len() == b.then_branch.roots.len() {
                score += 1;
            }

            if a.then_branch.node_paths.len() == b.then_branch.node_paths.len() {
                score += 1;
            }

            if a.then_branch.attr_paths.len() == b.then_branch.attr_paths.len() {
                score += 1;
            }

            score
        }

        _ => 0,
    }
}

/// todo: write some tests
pub fn score_attribute(old_attr: &Attribute, new_attr: &Attribute) -> usize {
    if old_attr.name != new_attr.name {
        return 0;
    }

    use AttributeValue::*;

    match (&old_attr.value, &new_attr.value) {
        // For literals, the value itself might change, but what's more important is the
        // structure of the literal. If the structure is the same, we can hotreload it
        // Ideally the value doesn't change, but we're hoping that our stack approach
        // Will prevent spurious reloads
        //
        // todo: maybe it's a good idea to modify the original in place?
        // todo: float to int is a little weird case that we can try to support better
        //       right now going from float to int or vice versa will cause a full rebuild
        //       which can get confusing. if we can figure out a way to hotreload this, that'd be great
        (AttrLiteral(left), AttrLiteral(right)) => {
            // We assign perfect matches for token resuse, to minimize churn on the renderer
            match (&left.value, &right.value) {
                // Quick shortcut if there's no change
                (HotLiteralType::Fmted(old), HotLiteralType::Fmted(new)) => {
                    if new == old {
                        return usize::MAX;
                    }

                    // We can remove formatted bits but we can't add them. The scoring here must
                    // realize that every bit of the new formatted segment must be in the old formatted segment
                    old.hr_score(new)
                }

                (HotLiteralType::Float(a), HotLiteralType::Float(b)) if a == b => usize::MAX,
                (HotLiteralType::Float(_), HotLiteralType::Float(_)) => 1,

                (HotLiteralType::Int(a), HotLiteralType::Int(b)) if a == b => usize::MAX,
                (HotLiteralType::Int(_), HotLiteralType::Int(_)) => 1,

                (HotLiteralType::Bool(a), HotLiteralType::Bool(b)) if a == b => usize::MAX,
                (HotLiteralType::Bool(_), HotLiteralType::Bool(_)) => 1,
                _ => 0,
            }
        }

        // If it's expression-type things, we give a perfect score if they match completely
        _ if old_attr == new_attr => usize::MAX,

        // If it's not a match, we give it a score of 0
        _ => 0,
    }
}

// #[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn score_components() {
        let a: BodyNode = syn::parse2(quote::quote! {
            for x in 0..1 {
                SomeComponent {
                    count: 19999123,
                    enabled: false,
                    title: "pxasd-5 {x}",
                    flot: 1233.5,
                    height: 100,
                    width: 500,
                    color: "reasdssdasd {x}",
                    handler: move |e| {
                        println!("clickeasdd!");
                    },
                    "sick!! asasdsd!lasdasasdasddasdkasjdlkasjdlk!! {x}"
                }
            }
        })
        .unwrap();

        let b: BodyNode = syn::parse2(quote::quote! {
            for x in 0..1 {
                SomeComponent {
                    count: 19999123,
                    enabled: false,
                    title: "pxasd-5 {x}",
                    flot: 1233.5,
                    height: 100,
                    width: 500,
                    color: "reasdssdasd {x}",
                    handler: move |e| {
                        println!("clickeasdd!");
                    },
                    "sick!! asasdsd!lasdasasdaasdasdsddasdkasjdlkasjdlk!! {x}"
                }
            }
        })
        .unwrap();

        let score = score_dynamic_node(&a, &b);
        assert_eq!(score, 4);
    }
}
