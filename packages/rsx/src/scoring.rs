#![cfg(feature = "hot_reload")]

use crate::{Attribute, AttributeValue, BodyNode, HotLiteralType, IfmtInput};

/// Take two nodes and return their similarity score
///
/// This is not normalized or anything, so longer nodes will have higher scores
pub fn score_dynamic_node(old_node: &BodyNode, new_node: &BodyNode) -> usize {
    use BodyNode::*;

    match (old_node, new_node) {
        (Element(_), Element(_)) => unreachable!("Elements are not dynamic nodes"),

        (Text(old), Text(new)) => {
            // We shouldn't be seeing static text nodes here
            assert!(!old.input.is_static() && !new.input.is_static());
            score_ifmt(&old.input, &new.input)
        }

        (RawExpr(old), RawExpr(new)) if old == new => usize::MAX,

        (Component(old), Component(new))
            if old.name == new.name
                && old.generics == new.generics
                && old.fields.len() == new.fields.len() =>
        {
            let mut score = 1;

            // todo: there might be a bug here where Idents and Strings will result in a match
            let mut left_fields = old.fields.iter().collect::<Vec<_>>();
            left_fields.sort_by(|a, b| a.name.to_string().cmp(&b.name.to_string()));

            let mut right_fields = new.fields.iter().collect::<Vec<_>>();
            right_fields.sort_by(|a, b| a.name.to_string().cmp(&b.name.to_string()));

            // Walk the attributes and score each one - if there's a zero we return zero
            // circuit if we there's an attribute mismatch that can't be hotreloaded
            for (left, right) in left_fields.iter().zip(right_fields.iter()) {
                let scored = match score_attribute(left, right) {
                    usize::MAX => 3,
                    0 => return 0,
                    a if a == usize::MAX - 1 => 2,
                    a => a,
                };

                score += scored;
            }

            score
        }

        (ForLoop(a), ForLoop(b)) if a.pat == b.pat && a.expr == b.expr => {
            // The bodies don't necessarily need to be the same, but we should throw some simple heuristics at them to
            // encourage proper selection. For now just double check the templates are roughly the same
            1 + (a.body.roots.len() == b.body.roots.len()) as usize
                + (a.body.node_paths.len() == b.body.node_paths.len()) as usize
                + (a.body.attr_paths.len() == b.body.attr_paths.len()) as usize
        }

        (IfChain(a), IfChain(b)) if a.cond == b.cond => {
            // The bodies don't necessarily need to be the same, but we should throw some simple heuristics at them to
            // encourage proper selection. For now just double check the templates are roughly the same
            1 + (a.then_branch.roots.len() == b.then_branch.roots.len()) as usize
                + (a.then_branch.node_paths.len() == b.then_branch.node_paths.len()) as usize
                + (a.then_branch.attr_paths.len() == b.then_branch.attr_paths.len()) as usize
        }

        _ => 0,
    }
}

pub fn score_attribute(old_attr: &Attribute, new_attr: &Attribute) -> usize {
    if old_attr.name != new_attr.name {
        return 0;
    }

    score_attr_value(&old_attr.value, &new_attr.value)
}

fn score_attr_value(old_attr: &AttributeValue, new_attr: &AttributeValue) -> usize {
    use AttributeValue::*;
    use HotLiteralType::*;

    match (&old_attr, &new_attr) {
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
            // We assign perfect matches for token reuse, to minimize churn on the renderer
            match (&left.value, &right.value) {
                // Quick shortcut if there's no change
                (Fmted(old), Fmted(new)) if old == new => usize::MAX,

                // We can remove formatted bits but we can't add them. The scoring here must
                // realize that every bit of the new formatted segment must be in the old formatted segment
                (Fmted(old), Fmted(new)) => score_ifmt(old, new),

                (Float(a), Float(b)) if a == b => usize::MAX,
                (Float(_), Float(_)) => 1,

                (Int(a), Int(b)) if a == b => usize::MAX,
                (Int(_), Int(_)) => 1,

                (Bool(a), Bool(b)) if a == b => usize::MAX,
                (Bool(_), Bool(_)) => 1,
                _ => 0,
            }
        }

        (
            AttrOptionalExpr {
                condition: cond_a,
                value: value_a,
            },
            AttrOptionalExpr {
                condition: cond_b,
                value: value_b,
            },
        ) if cond_a == cond_b => {
            // If the condition is the same, we can hotreload it
            score_attr_value(value_a, value_b)
        }

        // todo: we should try and score recursively if we can - templates need to propagate up their
        // scores. That would lead to a time complexity explosion but can be helpful in some cases.
        //
        // If it's expression-type things, we give a perfect score if they match completely
        _ if old_attr == new_attr => usize::MAX,

        // If it's not a match, we give it a score of 0
        _ => 0,
    }
}

pub fn score_ifmt(old: &IfmtInput, new: &IfmtInput) -> usize {
    // If they're the same by source, return max
    if old == new {
        return usize::MAX;
    }

    // Default score to 1 - an ifmt with no dynamic segments still technically has a score of 1
    // since it's not disqualified, but it's not a perfect match
    let mut score = 1;
    let mut l_freq_map = old.dynamic_seg_frequency_map();

    // Pluck out the dynamic segments from the other input
    for seg in new.dynamic_segments() {
        let Some(ct) = l_freq_map.get_mut(seg) else {
            return 0;
        };

        *ct -= 1;

        if *ct == 0 {
            l_freq_map.remove(seg);
        }

        score += 1;
    }

    // If there's nothing remaining - a perfect match - return max -1
    // We compared the sources to start, so we know they're different in some way
    if l_freq_map.is_empty() {
        usize::MAX - 1
    } else {
        score
    }
}

#[cfg(test)]
mod tests {
    use crate::PopVec;

    use super::*;
    use quote::quote;
    use syn::parse2;

    #[test]
    fn score_components() {
        let a: BodyNode = parse2(quote! {
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

        let b: BodyNode = parse2(quote! {
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

    #[test]
    fn score_attributes() {
        let left: Attribute = parse2(quote! { attr: 123 }).unwrap();
        let right: Attribute = parse2(quote! { attr: 123 }).unwrap();
        assert_eq!(score_attribute(&left, &right), usize::MAX);

        let left: Attribute = parse2(quote! { attr: 123 }).unwrap();
        let right: Attribute = parse2(quote! { attr: 456 }).unwrap();
        assert_eq!(score_attribute(&left, &right), 1);

        // almost a perfect match
        let left: Attribute = parse2(quote! { class: if count > 3 { "blah {abc}" } }).unwrap();
        let right: Attribute = parse2(quote! { class: if count > 3 { "other {abc}" } }).unwrap();
        assert_eq!(score_attribute(&left, &right), usize::MAX - 1);
    }

    /// Ensure the scoring algorithm works
    ///
    /// - usize::MAX is return for perfect overlap
    /// - 0 is returned when the right case has segments not found in the first
    /// - a number for the other cases where there is some non-perfect overlap
    #[test]
    fn ifmt_scoring() {
        let left: IfmtInput = "{abc} {def}".parse().unwrap();
        let right: IfmtInput = "{abc}".parse().unwrap();
        assert_eq!(score_ifmt(&left, &right), 2);

        let left: IfmtInput = "{abc} {def}".parse().unwrap();
        let right: IfmtInput = "{abc} {def}".parse().unwrap();
        assert_eq!(score_ifmt(&left, &right), usize::MAX);

        let left: IfmtInput = "{abc} {def}".parse().unwrap();
        let right: IfmtInput = "{abc} {ghi}".parse().unwrap();
        assert_eq!(score_ifmt(&left, &right), 0);

        let left: IfmtInput = "{abc} {def}".parse().unwrap();
        let right: IfmtInput = "{abc} {def} {ghi}".parse().unwrap();
        assert_eq!(score_ifmt(&left, &right), 0);

        let left: IfmtInput = "{abc} {def} {ghi}".parse().unwrap();
        let right: IfmtInput = "{abc} {def}".parse().unwrap();
        assert_eq!(score_ifmt(&left, &right), 3);

        let left: IfmtInput = "{abc}".parse().unwrap();
        let right: IfmtInput = "{abc} {def}".parse().unwrap();
        assert_eq!(score_ifmt(&left, &right), 0);

        let left: IfmtInput = "{abc} {abc} {def}".parse().unwrap();
        let right: IfmtInput = "{abc} {def}".parse().unwrap();
        assert_eq!(score_ifmt(&left, &right), 3);

        let left: IfmtInput = "{abc} {abc}".parse().unwrap();
        let right: IfmtInput = "{abc} {abc}".parse().unwrap();
        assert_eq!(score_ifmt(&left, &right), usize::MAX);

        let left: IfmtInput = "{abc} {def}".parse().unwrap();
        let right: IfmtInput = "{hij}".parse().unwrap();
        assert_eq!(score_ifmt(&left, &right), 0);

        let left: IfmtInput = "{abc}".parse().unwrap();
        let right: IfmtInput = "thing {abc}".parse().unwrap();
        assert_eq!(score_ifmt(&left, &right), usize::MAX - 1);

        let left: IfmtInput = "thing {abc}".parse().unwrap();
        let right: IfmtInput = "{abc}".parse().unwrap();
        assert_eq!(score_ifmt(&left, &right), usize::MAX - 1);

        let left: IfmtInput = "{abc} {def}".parse().unwrap();
        let right: IfmtInput = "thing {abc}".parse().unwrap();
        assert_eq!(score_ifmt(&left, &right), 2);
    }

    #[test]
    fn stack_scoring() {
        let stack: PopVec<IfmtInput> = PopVec::new(
            vec![
                "{abc} {def}".parse().unwrap(),
                "{def}".parse().unwrap(),
                "{hij}".parse().unwrap(),
            ]
            .into_iter(),
        );

        let tests = vec![
            "thing {def}".parse().unwrap(),
            "thing {abc}".parse().unwrap(),
            "thing {hij}".parse().unwrap(),
        ];

        for item in tests {
            let score = stack.highest_score(|f| score_ifmt(f, &item));

            dbg!(item, score);
        }
    }
}
