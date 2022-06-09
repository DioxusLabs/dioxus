mod injection {
    use std::collections::HashMap;

    use crate::props::injection::{make_segment, Nth, Segments, SelectorMode, Selectors};

    const VALID_SELECTORS: &str = "valid Selectors";
    const VALID_SEGMENT: &str = "valid Segment";

    mod selectors {
        use std::str::FromStr;

        use crate::props::injection::{Nth, SelectorMode, Selectors};
        use crate::tests::props::injection::expected_selectors2;

        use super::{expected_selectors, VALID_SELECTORS};

        #[test]
        fn selectors_duplicate() {
            let actual = Selectors::from_str("div; div");
            let expected = Err(String::from("duplicate selector 'div'"));

            assert_eq!(expected, actual, "duped");

            let actual = Selectors::from_str("div > :not(div,MyButton); div > :not[MyButton,div]");
            let expected = Err(String::from(
                "duplicate selector 'div > :not[MyButton,div]'",
            ));

            assert_eq!(expected, actual, "duped - nth not");

            let actual = Selectors::from_str("div > input:nth-child(2,5); div > input@[2, 5]")
                .expect(VALID_SELECTORS);

            let expected = expected_selectors2(vec![
                vec![
                    ("div", SelectorMode::Child, Nth::All),
                    ("input", SelectorMode::Child, Nth::Nth(vec![2, 5].into())),
                ],
                vec![
                    ("div", SelectorMode::Child, Nth::All),
                    ("input", SelectorMode::TypeOf, Nth::Nth(vec![2, 5].into())),
                ],
            ]);

            assert_eq!(expected, actual, "not duped");
        }

        #[test]
        fn selectors_duplicate_child() {
            let actual = Selectors::from_str("div > input:nth-child(2,5); div > input:[2,5]");
            let expected = Err(String::from("duplicate selector 'div > input:[2,5]'"));

            assert_eq!(expected, actual, "duped - nth child");

            let actual = Selectors::from_str("div > input:nth-child(2,5); div > input:[5,2]");
            let expected = Err(String::from("duplicate selector 'div > input:[5,2]'"));

            assert_eq!(expected, actual, "duped equivalent - nth child");

            let actual =
                Selectors::from_str("div > input:nth-last-child(2,5); div > input:last[2,5]");
            let expected = Err(String::from("duplicate selector 'div > input:last[2,5]'"));

            assert_eq!(expected, actual, "duped - nth last child");

            let actual =
                Selectors::from_str("div > input:nth-last-child(2,5); div > input:last[5,2]");
            let expected = Err(String::from("duplicate selector 'div > input:last[5,2]'"));

            assert_eq!(expected, actual, "duped equivalent - nth last child");

            let actual = Selectors::from_str("div > input:nth-child(2..5); div > input:[2..5]");
            let expected = Err(String::from("duplicate selector 'div > input:[2..5]'"));

            assert_eq!(expected, actual, "duped - nth child range");

            let actual = Selectors::from_str("div > input:nth-child(..5); div > input:[..5]");
            let expected = Err(String::from("duplicate selector 'div > input:[..5]'"));

            assert_eq!(expected, actual, "duped - nth child range to");

            let actual = Selectors::from_str("div > input:nth-child(..=5); div > input:[..6]");
            let expected = Err(String::from("duplicate selector 'div > input:[..6]'"));

            assert_eq!(expected, actual, "duped - nth child range to inclusive end");

            let actual = Selectors::from_str("div > input:nth-child(2..); div > input:[2..]");
            let expected = Err(String::from("duplicate selector 'div > input:[2..]'"));

            assert_eq!(expected, actual, "duped - nth child range from");
        }

        #[test]
        fn selectors_duplicate_of_type() {
            let actual = Selectors::from_str("div > input:nth-of-type(2,5); div > input@[2,5]");
            let expected = Err(String::from("duplicate selector 'div > input@[2,5]'"));

            assert_eq!(expected, actual, "duped - nth of type");

            let actual = Selectors::from_str("div > input:nth-of-type(2,5); div > input@[5,2]");
            let expected = Err(String::from("duplicate selector 'div > input@[5,2]'"));

            assert_eq!(expected, actual, "duped equivalent - nth of type");

            let actual =
                Selectors::from_str("div > input:nth-last-of-type(2,5); div > input@last[2,5]");
            let expected = Err(String::from("duplicate selector 'div > input@last[2,5]'"));

            assert_eq!(expected, actual, "duped - nth last of type");

            let actual =
                Selectors::from_str("div > input:nth-last-of-type(2,5); div > input@last[5,2]");
            let expected = Err(String::from("duplicate selector 'div > input@last[5,2]'"));

            assert_eq!(expected, actual, "duped equivalent - nth last of type");

            let actual = Selectors::from_str("div > input:nth-of-type(2..5); div > input@[2..5]");
            let expected = Err(String::from("duplicate selector 'div > input@[2..5]'"));

            assert_eq!(expected, actual, "duped - nth of type range");

            let actual = Selectors::from_str("div > input:nth-of-type(..5); div > input@[..5]");
            let expected = Err(String::from("duplicate selector 'div > input@[..5]'"));

            assert_eq!(expected, actual, "duped - nth of type range to");

            let actual = Selectors::from_str("div > input:nth-of-type(..=5); div > input@[..6]");
            let expected = Err(String::from("duplicate selector 'div > input@[..6]'"));

            assert_eq!(
                expected, actual,
                "duped - nth of type range to inclusive end"
            );

            let actual = Selectors::from_str("div > input:nth-of-type(2..); div > input@[2..]");
            let expected = Err(String::from("duplicate selector 'div > input@[2..]'"));

            assert_eq!(expected, actual, "duped - nth of type range from");
        }

        #[test]
        fn selector_basic() {
            let actual = Selectors::from_str("div").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::Child, Nth::All)]);

            assert_eq!(expected, actual, "single element");

            let actual = Selectors::from_str("MyButton").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::Child, Nth::All)]);

            assert_eq!(expected, actual, "single component");

            let actual = Selectors::from_str("div > MyButton").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![
                ("div", SelectorMode::Child, Nth::All),
                ("MyButton", SelectorMode::Child, Nth::All),
            ]);

            assert_eq!(expected, actual, "multiple");
        }

        #[test]
        fn selector_invalid_html_element_or_component() {
            let actual = Selectors::from_str("div > butt on");
            let expected = Err(String::from(
                "exception parsing selector 'div > butt on'; 'butt on' is an invalid html tag",
            ));

            assert_eq!(expected, actual, "html element");

            let actual = Selectors::from_str("div > butt_on");
            let expected = Err(String::from(
                "exception parsing selector 'div > butt_on'; 'butt_on' is an invalid html tag",
            ));

            assert_eq!(expected, actual, "html element w/underscore");

            let actual = Selectors::from_str("div > My Butt on");
            let expected = Err(String::from("exception parsing selector 'div > My Butt on'; 'My Butt on' is an invalid component name"));

            assert_eq!(expected, actual, "custom component");

            let actual = Selectors::from_str("div > myButton");
            let expected = Err(String::from("exception parsing selector 'div > myButton'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "custom component w/lowercase first char");

            let actual = Selectors::from_str("div > My_Button");
            let expected = Err(String::from("exception parsing selector 'div > My_Button'; 'My_Button' is an invalid component name"));

            assert_eq!(expected, actual, "custom component w/underscore");
        }
    }

    mod rusty_selectors {
        use std::ops::{Range, RangeFrom, RangeTo};
        use std::str::FromStr;

        use crate::props::injection::{Nth, SelectorMode, Selectors};

        use super::{expected_selectors, VALID_SELECTORS};

        #[test]
        fn rusty_selector_not_element() {
            let actual = Selectors::from_str(":not[div , MyButton]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "*",
                SelectorMode::Child,
                Nth::Not(vec![String::from("div"), String::from("MyButton")].into()),
            )]);

            assert_eq!(expected, actual, "valid elements; spaces ok");

            let actual = Selectors::from_str(":not[div,,MyButton]");
            let expected = Err(String::from(
                "exception parsing selector ':not[div,,MyButton]'; '' an empty element is invalid",
            ));

            assert_eq!(expected, actual, "blank not accepted");

            let actual = Selectors::from_str(":not[div,myButton]");
            let expected = Err(String::from("exception parsing selector ':not[div,myButton]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid elements");
        }

        #[test]
        fn rusty_selector_only_child_element() {
            let actual = Selectors::from_str("div:only").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::Child, Nth::Only)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:only").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::Child, Nth::Only)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton:only");
            let expected = Err(String::from("exception parsing selector 'myButton:only'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_only_of_type_element() {
            let actual = Selectors::from_str("div@only").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::TypeOf, Nth::Only)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton@only").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::TypeOf, Nth::Only)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton@only");
            let expected = Err(String::from("exception parsing selector 'myButton@only'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_invalid_nth_child() {
            let actual = Selectors::from_str(":foo[5]");
            let expected = Err(String::from(
                "exception parsing selector ':foo[5]'; 'foo[5]' is not a valid nth selector value",
            ));

            assert_eq!(expected, actual, "invalid nth");

            let actual = Selectors::from_str(":[5");
            let expected = Err(String::from(
                "exception parsing selector ':[5'; '[5' invalid nth expression",
            ));

            assert_eq!(expected, actual, "incomplete nth");

            let actual = Selectors::from_str(":not[div");
            let expected = Err(String::from(
                "exception parsing selector ':not[div'; 'not[div' invalid not expression",
            ));

            assert_eq!(expected, actual, "incomplete not");

            let actual = Selectors::from_str(":last[3");
            let expected = Err(String::from(
                "exception parsing selector ':last[3'; 'last[3' invalid nth last expression",
            ));

            assert_eq!(expected, actual, "incomplete last");

            let actual = Selectors::from_str(":[2..");
            let expected = Err(String::from(
                "exception parsing selector ':[2..'; '[2..' invalid nth range expression",
            ));

            assert_eq!(expected, actual, "incomplete range");

            let actual = Selectors::from_str(":[..5");
            let expected = Err(String::from(
                "exception parsing selector ':[..5'; '[..5' invalid nth range to expression",
            ));

            assert_eq!(expected, actual, "incomplete range to");

            let actual = Selectors::from_str(":[3n1");
            let expected = Err(String::from(
                "exception parsing selector ':[3n1'; '[3n1' invalid nth every n expression",
            ));

            assert_eq!(expected, actual, "incomplete every n");

            let actual = Selectors::from_str(":last[3n1");
            let expected = Err(String::from("exception parsing selector ':last[3n1'; 'last[3n1' invalid nth last every n expression"));

            assert_eq!(expected, actual, "incomplete last every n");
        }

        #[test]
        fn rusty_selector_nth_child_all() {
            let actual = Selectors::from_str("div:[..]");
            let expected = Err(String::from("exception parsing selector 'div:[..]'; Unnecessary 'all' range selector, remove ':[..]'"));

            assert_eq!(expected, actual, "_element");

            let actual = Selectors::from_str("MyButton:[..]");
            let expected = Err(String::from("exception parsing selector 'MyButton:[..]'; Unnecessary 'all' range selector, remove ':[..]'"));

            assert_eq!(expected, actual, "_component");
        }

        #[test]
        fn rusty_selector_nth_child() {
            let actual = Selectors::from_str("div:[0]").expect(VALID_SELECTORS);
            let expected =
                expected_selectors(vec![("div", SelectorMode::Child, Nth::Nth(vec![0].into()))]);

            assert_eq!(expected, actual, "first nth - element");

            let actual = Selectors::from_str("div:[3]").expect(VALID_SELECTORS);
            let expected =
                expected_selectors(vec![("div", SelectorMode::Child, Nth::Nth(vec![3].into()))]);

            assert_eq!(expected, actual, "not first nth - element");

            let actual = Selectors::from_str("MyButton:[0]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::Nth(vec![0].into()),
            )]);

            assert_eq!(expected, actual, "first nth - component");

            let actual = Selectors::from_str("MyButton:[3]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::Nth(vec![3].into()),
            )]);

            assert_eq!(expected, actual, "not first nth - component");

            let actual = Selectors::from_str("MyButton:[]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[]'; value can not be blank",
            ));

            assert_eq!(expected, actual, "blank index");

            let actual = Selectors::from_str("MyButton:[-3]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[-3]'; invalid value '-3'",
            ));

            assert_eq!(expected, actual, "negative index");

            let actual = Selectors::from_str("MyButton:[three]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[three]'; invalid value 'three'",
            ));

            assert_eq!(expected, actual, "invalid index");

            let actual = Selectors::from_str("myButton:[3]");
            let expected = Err(String::from("exception parsing selector 'myButton:[3]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_child_collection() {
            let actual = Selectors::from_str("div:[0,3,8]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::Nth(vec![0, 3, 8].into()),
            )]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:[0,3,8]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::Nth(vec![0, 3, 8].into()),
            )]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("MyButton:[0,3,,8]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[0,3,,8]'; value can not be blank",
            ));

            assert_eq!(expected, actual, "blank index");

            let actual = Selectors::from_str("MyButton:[0,3,-8]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[0,3,-8]'; invalid value '-8'",
            ));

            assert_eq!(expected, actual, "negative index");

            let actual = Selectors::from_str("MyButton:[zero,3,8]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[zero,3,8]'; invalid value 'zero'",
            ));

            assert_eq!(expected, actual, "invalid index");

            let actual = Selectors::from_str("myButton:[0,3,8]");
            let expected = Err(String::from("exception parsing selector 'myButton:[0,3,8]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_child_even() {
            let actual = Selectors::from_str("div:even").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::Child, Nth::Even)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:even").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::Child, Nth::Even)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton:even");
            let expected = Err(String::from("exception parsing selector 'myButton:even'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_child_odd() {
            let actual = Selectors::from_str("div:odd").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::Child, Nth::Odd)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:odd").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::Child, Nth::Odd)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton:odd");
            let expected = Err(String::from("exception parsing selector 'myButton:odd'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_first_child() {
            let actual = Selectors::from_str("div:first").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::Child, Nth::First)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:first").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::Child, Nth::First)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton:first");
            let expected = Err(String::from("exception parsing selector 'myButton:first'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_last_child() {
            let actual = Selectors::from_str("div:last").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::Child, Nth::Last)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:last").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::Child, Nth::Last)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton:last");
            let expected = Err(String::from("exception parsing selector 'myButton:last'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_last_child() {
            let actual = Selectors::from_str("div:last[0]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::NthLast(vec![0].into()),
            )]);

            assert_eq!(expected, actual, "last nth - element");

            let actual = Selectors::from_str("div:last[3]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::NthLast(vec![3].into()),
            )]);

            assert_eq!(expected, actual, "not last nth - element");

            let actual = Selectors::from_str("MyButton:last[0]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::NthLast(vec![0].into()),
            )]);

            assert_eq!(expected, actual, "last nth - component");

            let actual = Selectors::from_str("MyButton:last[3]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::NthLast(vec![3].into()),
            )]);

            assert_eq!(expected, actual, "not last nth - component");

            let actual = Selectors::from_str("MyButton:last[]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:last[]'; value can not be blank",
            ));

            assert_eq!(expected, actual, "blank index");

            let actual = Selectors::from_str("MyButton:last[-3]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:last[-3]'; invalid value '-3'",
            ));

            assert_eq!(expected, actual, "negative index");

            let actual = Selectors::from_str("MyButton:last[three]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:last[three]'; invalid value 'three'",
            ));

            assert_eq!(expected, actual, "invalid index");

            let actual = Selectors::from_str("myButton:last[0]");
            let expected = Err(String::from("exception parsing selector 'myButton:last[0]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_last_child_collection() {
            let actual = Selectors::from_str("div:last[0,3,8]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::NthLast(vec![0, 3, 8].into()),
            )]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:last[0,3,8]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::NthLast(vec![0, 3, 8].into()),
            )]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("MyButton:last[0,3,,8]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:last[0,3,,8]'; value can not be blank",
            ));

            assert_eq!(expected, actual, "blank index");

            let actual = Selectors::from_str("MyButton:last[0,3,-8]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:last[0,3,-8]'; invalid value '-8'",
            ));

            assert_eq!(expected, actual, "negative index");

            let actual = Selectors::from_str("MyButton:last[zero,3,8]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:last[zero,3,8]'; invalid value 'zero'",
            ));

            assert_eq!(expected, actual, "invalid index");

            let actual = Selectors::from_str("myButton:last[0,3,8]");
            let expected = Err(String::from("exception parsing selector 'myButton:last[0,3,8]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_child_range() {
            let actual = Selectors::from_str("div:[2..5]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::Range(Range { start: 2, end: 5 }),
            )]);

            assert_eq!(expected, actual, "not inclusive - element");

            let actual = Selectors::from_str("div:[2..=5]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::Range(Range { start: 2, end: 6 }),
            )]);

            assert_eq!(expected, actual, "inclusive - element");

            let actual = Selectors::from_str("MyButton:[2..5]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::Range(Range { start: 2, end: 5 }),
            )]);

            assert_eq!(expected, actual, "not inclusive - component");

            let actual = Selectors::from_str("MyButton:[2..=5]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::Range(Range { start: 2, end: 6 }),
            )]);

            assert_eq!(expected, actual, "inclusive - component");

            let actual = Selectors::from_str("MyButton:[5..2]");
            let expected = Err(String::from("exception parsing selector 'MyButton:[5..2]'; range start cannot be more than end; 2 < 5"));

            assert_eq!(expected, actual, "start more than end");

            let actual = Selectors::from_str("MyButton:[5..=2]");
            let expected = Err(String::from("exception parsing selector 'MyButton:[5..=2]'; range start cannot be more than inclusive end; =2 < 5"));

            assert_eq!(expected, actual, "start more than inclusive end");

            let actual = Selectors::from_str("MyButton:[-2..5]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[-2..5]'; '-2' is not a valid start value",
            ));

            assert_eq!(expected, actual, "negative start");

            let actual = Selectors::from_str("MyButton:[two..5]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[two..5]'; 'two' is not a valid start value",
            ));

            assert_eq!(expected, actual, "invalid start");

            let actual = Selectors::from_str("MyButton:[2..-5]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[2..-5]'; '-5' is not a valid end value",
            ));

            assert_eq!(expected, actual, "negative end");

            let actual = Selectors::from_str("MyButton:[2..five]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[2..five]'; 'five' is not a valid end value",
            ));

            assert_eq!(expected, actual, "invalid end");

            let actual = Selectors::from_str("MyButton:[2..=-5]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[2..=-5]'; '=-5' is not a valid end value",
            ));

            assert_eq!(expected, actual, "negative inclusive end");

            let actual = Selectors::from_str("MyButton:[2..=five]");
            let expected = Err(String::from("exception parsing selector 'MyButton:[2..=five]'; '=five' is not a valid end value"));

            assert_eq!(expected, actual, "invalid inclusive end");

            let actual = Selectors::from_str("myButton:[2..5]");
            let expected = Err(String::from("exception parsing selector 'myButton:[2..5]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_child_range_to() {
            let actual = Selectors::from_str("div:[..5]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::RangeTo(RangeTo { end: 5 }),
            )]);

            assert_eq!(expected, actual, "not inclusive - element");

            let actual = Selectors::from_str("div:[..=5]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::RangeTo(RangeTo { end: 6 }),
            )]);

            assert_eq!(expected, actual, "inclusive - element");

            let actual = Selectors::from_str("MyButton:[..-5]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[..-5]'; '-5' is not a valid end value",
            ));

            assert_eq!(expected, actual, "negative not inclusive end");

            let actual = Selectors::from_str("MyButton:[..five]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[..five]'; 'five' is not a valid end value",
            ));

            assert_eq!(expected, actual, "invalid not inclusive end");

            let actual = Selectors::from_str("MyButton:[..=-5]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[..=-5]'; '=-5' is not a valid end value",
            ));

            assert_eq!(expected, actual, "negative inclusive end");

            let actual = Selectors::from_str("MyButton:[..=five]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[..=five]'; '=five' is not a valid end value",
            ));

            assert_eq!(expected, actual, "invalid inclusive end");

            let actual = Selectors::from_str("myButton:[..5]");
            let expected = Err(String::from("exception parsing selector 'myButton:[..5]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_child_range_from() {
            let actual = Selectors::from_str("div:[2..]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::RangeFrom(RangeFrom { start: 2 }),
            )]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:[2..]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::RangeFrom(RangeFrom { start: 2 }),
            )]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("MyButton:[-2..]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[-2..]'; '-2' is not a valid start value",
            ));

            assert_eq!(expected, actual, "negative start");

            let actual = Selectors::from_str("MyButton:[two..]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[two..]'; 'two' is not a valid start value",
            ));

            assert_eq!(expected, actual, "invalid start");

            let actual = Selectors::from_str("myButton:[2..]");
            let expected = Err(String::from("exception parsing selector 'myButton:[2..]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_child_every_n() {
            let actual = Selectors::from_str("div:[3n]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::EveryN {
                    frequency: 3,
                    offset: 0,
                },
            )]);

            assert_eq!(expected, actual, "no offset - element");

            let actual = Selectors::from_str("div:[3n+2]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::EveryN {
                    frequency: 3,
                    offset: 2,
                },
            )]);

            assert_eq!(expected, actual, "positive offset - element");

            let actual = Selectors::from_str("div:[3n-2]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::EveryN {
                    frequency: 3,
                    offset: -2,
                },
            )]);

            assert_eq!(expected, actual, "negative offset - element");

            let actual = Selectors::from_str("MyButton:[3n]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::EveryN {
                    frequency: 3,
                    offset: 0,
                },
            )]);

            assert_eq!(expected, actual, "no offset - component");

            let actual = Selectors::from_str("MyButton:[3n+2]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::EveryN {
                    frequency: 3,
                    offset: 2,
                },
            )]);

            assert_eq!(expected, actual, "positive offset - component");

            let actual = Selectors::from_str("MyButton:[3n-2]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::EveryN {
                    frequency: 3,
                    offset: -2,
                },
            )]);

            assert_eq!(expected, actual, "negative offset - component");

            let actual = Selectors::from_str("MyButton:[-3n-2]");
            let expected = Err(String::from("exception parsing selector 'MyButton:[-3n-2]'; '-3' is not a valid frequency value"));

            assert_eq!(expected, actual, "negative frequency");

            let actual = Selectors::from_str("MyButton:[threen-2]");
            let expected = Err(String::from("exception parsing selector 'MyButton:[threen-2]'; 'three' is not a valid frequency value"));

            assert_eq!(expected, actual, "invalid frequency");

            let actual = Selectors::from_str("MyButton:[3ntwo]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:[3ntwo]'; 'two' is not a valid offset value",
            ));

            assert_eq!(expected, actual, "invalid offset");

            let actual = Selectors::from_str("myButton:[3n-2]");
            let expected = Err(String::from("exception parsing selector 'myButton:[3n-2]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_invalid_nth_of_type() {
            let actual = Selectors::from_str("@foo[5]");
            let expected = Err(String::from(
                "exception parsing selector '@foo[5]'; 'foo[5]' is not a valid nth selector value",
            ));

            assert_eq!(expected, actual, "invalid nth");

            let actual = Selectors::from_str("@[5");
            let expected = Err(String::from(
                "exception parsing selector '@[5'; '[5' invalid nth expression",
            ));

            assert_eq!(expected, actual, "incomplete nth");

            let actual = Selectors::from_str("@not[div");
            let expected = Err(String::from(
                "exception parsing selector '@not[div'; 'not[div' invalid not expression",
            ));

            assert_eq!(expected, actual, "incomplete not");

            let actual = Selectors::from_str("@last[3");
            let expected = Err(String::from(
                "exception parsing selector '@last[3'; 'last[3' invalid nth last expression",
            ));

            assert_eq!(expected, actual, "incomplete last");

            let actual = Selectors::from_str("@[2..");
            let expected = Err(String::from(
                "exception parsing selector '@[2..'; '[2..' invalid nth range expression",
            ));

            assert_eq!(expected, actual, "incomplete range");

            let actual = Selectors::from_str("@[..5");
            let expected = Err(String::from(
                "exception parsing selector '@[..5'; '[..5' invalid nth range to expression",
            ));

            assert_eq!(expected, actual, "incomplete range to");

            let actual = Selectors::from_str("@[3n1");
            let expected = Err(String::from(
                "exception parsing selector '@[3n1'; '[3n1' invalid nth every n expression",
            ));

            assert_eq!(expected, actual, "incomplete every n");

            let actual = Selectors::from_str("@last[3n1");
            let expected = Err(String::from("exception parsing selector '@last[3n1'; 'last[3n1' invalid nth last every n expression"));

            assert_eq!(expected, actual, "incomplete last every n");
        }

        #[test]
        fn rusty_selector_nth_of_type_all() {
            let actual = Selectors::from_str("div@[..]");
            let expected = Err(String::from("exception parsing selector 'div@[..]'; Unnecessary 'all' range selector, remove '@[..]'"));

            assert_eq!(expected, actual, "_element");

            let actual = Selectors::from_str("MyButton@[..]");
            let expected = Err(String::from("exception parsing selector 'MyButton@[..]'; Unnecessary 'all' range selector, remove '@[..]'"));

            assert_eq!(expected, actual, "_component");
        }

        #[test]
        fn rusty_selector_nth_of_type() {
            let actual = Selectors::from_str("div@[0]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::Nth(vec![0].into()),
            )]);

            assert_eq!(expected, actual, "first nth - element");

            let actual = Selectors::from_str("div@[3]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::Nth(vec![3].into()),
            )]);

            assert_eq!(expected, actual, "not first nth - element");

            let actual = Selectors::from_str("MyButton@[0]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::Nth(vec![0].into()),
            )]);

            assert_eq!(expected, actual, "first nth - component");

            let actual = Selectors::from_str("MyButton@[3]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::Nth(vec![3].into()),
            )]);

            assert_eq!(expected, actual, "not first nth - component");

            let actual = Selectors::from_str("MyButton@[]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[]'; value can not be blank",
            ));

            assert_eq!(expected, actual, "blank index");

            let actual = Selectors::from_str("MyButton@[-3]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[-3]'; invalid value '-3'",
            ));

            assert_eq!(expected, actual, "negative index");

            let actual = Selectors::from_str("MyButton@[three]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[three]'; invalid value 'three'",
            ));

            assert_eq!(expected, actual, "invalid index");

            let actual = Selectors::from_str("myButton@[3]");
            let expected = Err(String::from("exception parsing selector 'myButton@[3]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_of_type_collection() {
            let actual = Selectors::from_str("div@[0,3,8]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::Nth(vec![0, 3, 8].into()),
            )]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton@[0,3,8]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::Nth(vec![0, 3, 8].into()),
            )]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("MyButton@[0,3,,8]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[0,3,,8]'; value can not be blank",
            ));

            assert_eq!(expected, actual, "blank index");

            let actual = Selectors::from_str("MyButton@[0,3,-8]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[0,3,-8]'; invalid value '-8'",
            ));

            assert_eq!(expected, actual, "negative index");

            let actual = Selectors::from_str("MyButton@[zero,3,8]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[zero,3,8]'; invalid value 'zero'",
            ));

            assert_eq!(expected, actual, "invalid index");

            let actual = Selectors::from_str("myButton@[0,3,8]");
            let expected = Err(String::from("exception parsing selector 'myButton@[0,3,8]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_of_type_even() {
            let actual = Selectors::from_str("div@even").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::TypeOf, Nth::Even)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton@even").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::TypeOf, Nth::Even)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton@even");
            let expected = Err(String::from("exception parsing selector 'myButton@even'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_of_type_odd() {
            let actual = Selectors::from_str("div@odd").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::TypeOf, Nth::Odd)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton@odd").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::TypeOf, Nth::Odd)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton@odd");
            let expected = Err(String::from("exception parsing selector 'myButton@odd'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_of_type_first() {
            let actual = Selectors::from_str("div@first").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::TypeOf, Nth::First)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton@first").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::TypeOf, Nth::First)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton@first");
            let expected = Err(String::from("exception parsing selector 'myButton@first'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_of_type_last() {
            let actual = Selectors::from_str("div@last").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::TypeOf, Nth::Last)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton@last").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::TypeOf, Nth::Last)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton@last");
            let expected = Err(String::from("exception parsing selector 'myButton@last'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_last_of_type() {
            let actual = Selectors::from_str("div@last[0]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::NthLast(vec![0].into()),
            )]);

            assert_eq!(expected, actual, "last nth - element");

            let actual = Selectors::from_str("div@last[3]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::NthLast(vec![3].into()),
            )]);

            assert_eq!(expected, actual, "not last nth - element");

            let actual = Selectors::from_str("MyButton@last[0]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::NthLast(vec![0].into()),
            )]);

            assert_eq!(expected, actual, "last nth - component");

            let actual = Selectors::from_str("MyButton@last[3]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::NthLast(vec![3].into()),
            )]);

            assert_eq!(expected, actual, "not last nth - component");

            let actual = Selectors::from_str("MyButton@last[]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@last[]'; value can not be blank",
            ));

            assert_eq!(expected, actual, "blank index");

            let actual = Selectors::from_str("MyButton@last[-3]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@last[-3]'; invalid value '-3'",
            ));

            assert_eq!(expected, actual, "negative index");

            let actual = Selectors::from_str("MyButton@last[three]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@last[three]'; invalid value 'three'",
            ));

            assert_eq!(expected, actual, "invalid index");

            let actual = Selectors::from_str("myButton@last[0]");
            let expected = Err(String::from("exception parsing selector 'myButton@last[0]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_last_of_type_collection() {
            let actual = Selectors::from_str("div@last[0,3,8]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::NthLast(vec![0, 3, 8].into()),
            )]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton@last[0,3,8]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::NthLast(vec![0, 3, 8].into()),
            )]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("MyButton@last[0,3,,8]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@last[0,3,,8]'; value can not be blank",
            ));

            assert_eq!(expected, actual, "blank index");

            let actual = Selectors::from_str("MyButton@last[0,3,-8]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@last[0,3,-8]'; invalid value '-8'",
            ));

            assert_eq!(expected, actual, "negative index");

            let actual = Selectors::from_str("MyButton@last[zero,3,8]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@last[zero,3,8]'; invalid value 'zero'",
            ));

            assert_eq!(expected, actual, "invalid index");

            let actual = Selectors::from_str("myButton@last[0,3,8]");
            let expected = Err(String::from("exception parsing selector 'myButton@last[0,3,8]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_of_type_range() {
            let actual = Selectors::from_str("div@[2..5]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::Range(Range { start: 2, end: 5 }),
            )]);

            assert_eq!(expected, actual, "not inclusive - element");

            let actual = Selectors::from_str("div@[2..=5]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::Range(Range { start: 2, end: 6 }),
            )]);

            assert_eq!(expected, actual, "inclusive - element");

            let actual = Selectors::from_str("MyButton@[2..5]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::Range(Range { start: 2, end: 5 }),
            )]);

            assert_eq!(expected, actual, "not inclusive - component");

            let actual = Selectors::from_str("MyButton@[2..=5]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::Range(Range { start: 2, end: 6 }),
            )]);

            assert_eq!(expected, actual, "inclusive - component");

            let actual = Selectors::from_str("MyButton@[5..2]");
            let expected = Err(String::from("exception parsing selector 'MyButton@[5..2]'; range start cannot be more than end; 2 < 5"));

            assert_eq!(expected, actual, "start more than end");

            let actual = Selectors::from_str("MyButton@[5..=2]");
            let expected = Err(String::from("exception parsing selector 'MyButton@[5..=2]'; range start cannot be more than inclusive end; =2 < 5"));

            assert_eq!(expected, actual, "start more than inclusive end");

            let actual = Selectors::from_str("MyButton@[-2..5]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[-2..5]'; '-2' is not a valid start value",
            ));

            assert_eq!(expected, actual, "negative start");

            let actual = Selectors::from_str("MyButton@[two..5]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[two..5]'; 'two' is not a valid start value",
            ));

            assert_eq!(expected, actual, "invalid start");

            let actual = Selectors::from_str("MyButton@[2..-5]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[2..-5]'; '-5' is not a valid end value",
            ));

            assert_eq!(expected, actual, "negative end");

            let actual = Selectors::from_str("MyButton@[2..five]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[2..five]'; 'five' is not a valid end value",
            ));

            assert_eq!(expected, actual, "invalid end");

            let actual = Selectors::from_str("MyButton@[2..=-5]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[2..=-5]'; '=-5' is not a valid end value",
            ));

            assert_eq!(expected, actual, "negative inclusive end");

            let actual = Selectors::from_str("MyButton@[2..=five]");
            let expected = Err(String::from("exception parsing selector 'MyButton@[2..=five]'; '=five' is not a valid end value"));

            assert_eq!(expected, actual, "invalid inclusive end");

            let actual = Selectors::from_str("myButton@[2..5]");
            let expected = Err(String::from("exception parsing selector 'myButton@[2..5]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_of_type_range_to() {
            let actual = Selectors::from_str("div@[..5]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::RangeTo(RangeTo { end: 5 }),
            )]);

            assert_eq!(expected, actual, "not inclusive - element");

            let actual = Selectors::from_str("div@[..=5]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::RangeTo(RangeTo { end: 6 }),
            )]);

            assert_eq!(expected, actual, "inclusive - element");

            let actual = Selectors::from_str("MyButton@[..5]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::RangeTo(RangeTo { end: 5 }),
            )]);

            assert_eq!(expected, actual, "not inclusive - component");

            let actual = Selectors::from_str("MyButton@[..=5]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::RangeTo(RangeTo { end: 6 }),
            )]);

            assert_eq!(expected, actual, "inclusive - component");

            let actual = Selectors::from_str("MyButton@[..-5]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[..-5]'; '-5' is not a valid end value",
            ));

            assert_eq!(expected, actual, "negative not inclusive end");

            let actual = Selectors::from_str("MyButton@[..five]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[..five]'; 'five' is not a valid end value",
            ));

            assert_eq!(expected, actual, "invalid not inclusive end");

            let actual = Selectors::from_str("MyButton@[..=-5]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[..=-5]'; '=-5' is not a valid end value",
            ));

            assert_eq!(expected, actual, "negative inclusive end");

            let actual = Selectors::from_str("MyButton@[..=five]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[..=five]'; '=five' is not a valid end value",
            ));

            assert_eq!(expected, actual, "invalid inclusive end");

            let actual = Selectors::from_str("myButton@[..5]");
            let expected = Err(String::from("exception parsing selector 'myButton@[..5]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_of_type_range_from() {
            let actual = Selectors::from_str("div@[2..]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::RangeFrom(RangeFrom { start: 2 }),
            )]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton@[2..]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::RangeFrom(RangeFrom { start: 2 }),
            )]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("MyButton@[-2..]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[-2..]'; '-2' is not a valid start value",
            ));

            assert_eq!(expected, actual, "negative start");

            let actual = Selectors::from_str("MyButton@[two..]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[two..]'; 'two' is not a valid start value",
            ));

            assert_eq!(expected, actual, "invalid start");

            let actual = Selectors::from_str("myButton@[2..]");
            let expected = Err(String::from("exception parsing selector 'myButton@[2..]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn rusty_selector_nth_of_type_every_n() {
            let actual = Selectors::from_str("div@[3n]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::EveryN {
                    frequency: 3,
                    offset: 0,
                },
            )]);

            assert_eq!(expected, actual, "no offset - element");

            let actual = Selectors::from_str("div@[3n+2]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::EveryN {
                    frequency: 3,
                    offset: 2,
                },
            )]);

            assert_eq!(expected, actual, "positive offset - element");

            let actual = Selectors::from_str("div@[3n-2]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::EveryN {
                    frequency: 3,
                    offset: -2,
                },
            )]);

            assert_eq!(expected, actual, "negative offset - element");

            let actual = Selectors::from_str("MyButton@[3n]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::EveryN {
                    frequency: 3,
                    offset: 0,
                },
            )]);

            assert_eq!(expected, actual, "no offset - component");

            let actual = Selectors::from_str("MyButton@[3n+2]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::EveryN {
                    frequency: 3,
                    offset: 2,
                },
            )]);

            assert_eq!(expected, actual, "positive offset - component");

            let actual = Selectors::from_str("MyButton@[3n-2]").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::EveryN {
                    frequency: 3,
                    offset: -2,
                },
            )]);

            assert_eq!(expected, actual, "negative offset - component");

            let actual = Selectors::from_str("MyButton@[-3n-2]");
            let expected = Err(String::from("exception parsing selector 'MyButton@[-3n-2]'; '-3' is not a valid frequency value"));

            assert_eq!(expected, actual, "negative frequency");

            let actual = Selectors::from_str("MyButton@[threen-2]");
            let expected = Err(String::from("exception parsing selector 'MyButton@[threen-2]'; 'three' is not a valid frequency value"));

            assert_eq!(expected, actual, "invalid frequency");

            let actual = Selectors::from_str("MyButton@[3ntwo]");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton@[3ntwo]'; 'two' is not a valid offset value",
            ));

            assert_eq!(expected, actual, "invalid offset");

            let actual = Selectors::from_str("myButton@[3n-2]");
            let expected = Err(String::from("exception parsing selector 'myButton@[3n-2]'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }
    }

    mod css_selectors {
        use std::ops::{Range, RangeFrom, RangeTo};
        use std::str::FromStr;

        use crate::props::injection::{Nth, SelectorMode, Selectors};

        use super::{expected_selectors, VALID_SELECTORS};

        #[test]
        fn css_selector_not_element() {
            let actual = Selectors::from_str(":not(div , MyButton)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "*",
                SelectorMode::Child,
                Nth::Not(vec![String::from("div"), String::from("MyButton")].into()),
            )]);

            assert_eq!(expected, actual, "valid elements; spaces ok");

            let actual = Selectors::from_str(":not(div,,MyButton)");
            let expected = Err(String::from(
                "exception parsing selector ':not(div,,MyButton)'; '' an empty element is invalid",
            ));

            assert_eq!(expected, actual, "blank not accepted");

            let actual = Selectors::from_str(":not(div,myButton)");
            let expected = Err(String::from("exception parsing selector ':not(div,myButton)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid elements");
        }

        #[test]
        fn css_selector_only_child_element() {
            let actual = Selectors::from_str("div:only-child").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::Child, Nth::Only)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:only-child").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::Child, Nth::Only)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton:only-child");
            let expected = Err(String::from("exception parsing selector 'myButton:only-child'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_only_of_type_element() {
            let actual = Selectors::from_str("div:only-of-type").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::TypeOf, Nth::Only)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:only-of-type").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::TypeOf, Nth::Only)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton:only-of-type");
            let expected = Err(String::from("exception parsing selector 'myButton:only-of-type'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_invalid() {
            let actual = Selectors::from_str(":foo(5)");
            let expected = Err(String::from("exception parsing selector ':foo(5)'; 'foo' is not a valid css selector expression"));

            assert_eq!(expected, actual, "invalid css");

            let actual = Selectors::from_str(":foo(5");
            let expected = Err(String::from("exception parsing selector ':foo(5'; 'foo(5' is not a valid css selector expression"));

            assert_eq!(expected, actual, "invalid/incomplete css");

            let actual = Selectors::from_str(":not(5");
            let expected = Err(String::from(
                "exception parsing selector ':not(5'; '(5' is not a valid not expression",
            ));

            assert_eq!(expected, actual, "invalid css");
        }

        #[test]
        fn css_selector_invalid_nth_child() {
            let actual = Selectors::from_str(":nth-child(even");
            let expected = Err(String::from("exception parsing selector ':nth-child(even'; '(even' is not a valid nth child even expression"));

            assert_eq!(expected, actual, "incomplete even");

            let actual = Selectors::from_str(":nth-child(odd");
            let expected = Err(String::from("exception parsing selector ':nth-child(odd'; '(odd' is not a valid nth child odd expression"));

            assert_eq!(expected, actual, "incomplete odd");

            let actual = Selectors::from_str(":nth-child(5");
            let expected = Err(String::from("exception parsing selector ':nth-child(5'; '(5' is not a valid nth child expression"));

            assert_eq!(expected, actual, "incomplete nth");

            let actual = Selectors::from_str(":not(div");
            let expected = Err(String::from(
                "exception parsing selector ':not(div'; '(div' is not a valid not expression",
            ));

            assert_eq!(expected, actual, "incomplete not");

            let actual = Selectors::from_str(":nth-last-child(3");
            let expected = Err(String::from("exception parsing selector ':nth-last-child(3'; '(3' is not a valid nth last child expression"));

            assert_eq!(expected, actual, "incomplete last");

            let actual = Selectors::from_str(":nth-child(..");
            let expected = Err(String::from("exception parsing selector ':nth-child(..'; Unnecessary/Invalid all '..' range selector, remove ':nth-child(..'"));

            assert_eq!(expected, actual, "unnecessary/invalid all");

            let actual = Selectors::from_str(":nth-child(2..5");
            let expected = Err(String::from("exception parsing selector ':nth-child(2..5'; '(2..5' is not a valid nth child range expression"));

            assert_eq!(expected, actual, "incomplete range");

            let actual = Selectors::from_str(":nth-child(2..");
            let expected = Err(String::from("exception parsing selector ':nth-child(2..'; '(2..' is not a valid nth child range from expression"));

            assert_eq!(expected, actual, "incomplete range from");

            let actual = Selectors::from_str(":nth-child(..5");
            let expected = Err(String::from("exception parsing selector ':nth-child(..5'; '(..5' is not a valid nth child range to expression"));

            assert_eq!(expected, actual, "incomplete range to");

            let actual = Selectors::from_str(":nth-child(3n1");
            let expected = Err(String::from("exception parsing selector ':nth-child(3n1'; '(3n1' is not a valid nth every nth child expression"));

            assert_eq!(expected, actual, "incomplete every n");

            let actual = Selectors::from_str(":nth-last-child(3n1");
            let expected = Err(String::from("exception parsing selector ':nth-last-child(3n1'; '(3n1' is not a valid nth last every nth child expression"));

            assert_eq!(expected, actual, "incomplete last every n");
        }

        #[test]
        fn css_selector_nth_child_all() {
            let actual = Selectors::from_str("div:nth-child(..)");
            let expected = Err(String::from("exception parsing selector 'div:nth-child(..)'; Unnecessary 'all' range selector, remove ':nth-child(..)'"));

            assert_eq!(expected, actual, "_element");

            let actual = Selectors::from_str("MyButton:nth-child(..)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(..)'; Unnecessary 'all' range selector, remove ':nth-child(..)'"));

            assert_eq!(expected, actual, "_component");
        }

        #[test]
        fn css_selector_nth_child() {
            let actual = Selectors::from_str("div:nth-child(0)").expect(VALID_SELECTORS);
            let expected =
                expected_selectors(vec![("div", SelectorMode::Child, Nth::Nth(vec![0].into()))]);

            assert_eq!(expected, actual, "first nth - element");

            let actual = Selectors::from_str("div:nth-child(3)").expect(VALID_SELECTORS);
            let expected =
                expected_selectors(vec![("div", SelectorMode::Child, Nth::Nth(vec![3].into()))]);

            assert_eq!(expected, actual, "not first nth - element");

            let actual = Selectors::from_str("MyButton:nth-child(0)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::Nth(vec![0].into()),
            )]);

            assert_eq!(expected, actual, "first nth - component");

            let actual = Selectors::from_str("MyButton:nth-child(3)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::Nth(vec![3].into()),
            )]);

            assert_eq!(expected, actual, "not first nth - component");

            let actual = Selectors::from_str("MyButton:nth-child()");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-child()'; value can not be blank",
            ));

            assert_eq!(expected, actual, "blank index");

            let actual = Selectors::from_str("MyButton:nth-child(-3)");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-child(-3)'; invalid value '-3'",
            ));

            assert_eq!(expected, actual, "negative index");

            let actual = Selectors::from_str("MyButton:nth-child(three)");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-child(three)'; invalid value 'three'",
            ));

            assert_eq!(expected, actual, "invalid index");

            let actual = Selectors::from_str("myButton:nth-child(3)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-child(3)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_child_collection() {
            let actual = Selectors::from_str("div:nth-child(0,3,8)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::Nth(vec![0, 3, 8].into()),
            )]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:nth-child(0,3,8)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::Nth(vec![0, 3, 8].into()),
            )]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("MyButton:nth-child(0,3,,8)");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-child(0,3,,8)'; value can not be blank",
            ));

            assert_eq!(expected, actual, "blank index");

            let actual = Selectors::from_str("MyButton:nth-child(0,3,-8)");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-child(0,3,-8)'; invalid value '-8'",
            ));

            assert_eq!(expected, actual, "negative index");

            let actual = Selectors::from_str("MyButton:nth-child(zero,3,8)");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-child(zero,3,8)'; invalid value 'zero'",
            ));

            assert_eq!(expected, actual, "invalid index");

            let actual = Selectors::from_str("myButton:nth-child(0,3,8)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-child(0,3,8)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_child_even() {
            let actual = Selectors::from_str("div:nth-child(even)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::Child, Nth::Even)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:nth-child(even)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::Child, Nth::Even)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton:nth-child(even)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-child(even)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_child_odd() {
            let actual = Selectors::from_str("div:nth-child(odd)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::Child, Nth::Odd)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:nth-child(odd)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::Child, Nth::Odd)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton:nth-child(odd)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-child(odd)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_first_child() {
            let actual = Selectors::from_str("div:first-child").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::Child, Nth::First)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:first-child").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::Child, Nth::First)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton:first-child");
            let expected = Err(String::from("exception parsing selector 'myButton:first-child'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_last_child() {
            let actual = Selectors::from_str("div:last-child").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::Child, Nth::Last)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:last-child").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::Child, Nth::Last)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton:last-child");
            let expected = Err(String::from("exception parsing selector 'myButton:last-child'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_last_child() {
            let actual = Selectors::from_str("div:nth-last-child(0)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::NthLast(vec![0].into()),
            )]);

            assert_eq!(expected, actual, "last nth - element");

            let actual = Selectors::from_str("div:nth-last-child(3)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::NthLast(vec![3].into()),
            )]);

            assert_eq!(expected, actual, "not last nth - element");

            let actual = Selectors::from_str("MyButton:nth-last-child(0)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::NthLast(vec![0].into()),
            )]);

            assert_eq!(expected, actual, "last nth - component");

            let actual = Selectors::from_str("MyButton:nth-last-child(3)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::NthLast(vec![3].into()),
            )]);

            assert_eq!(expected, actual, "not last nth - component");

            let actual = Selectors::from_str("MyButton:nth-last-child()");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-last-child()'; value can not be blank",
            ));

            assert_eq!(expected, actual, "blank index");

            let actual = Selectors::from_str("MyButton:nth-last-child(-3)");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-last-child(-3)'; invalid value '-3'",
            ));

            assert_eq!(expected, actual, "negative index");

            let actual = Selectors::from_str("MyButton:nth-last-child(three)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-last-child(three)'; invalid value 'three'"));

            assert_eq!(expected, actual, "invalid index");

            let actual = Selectors::from_str("myButton:nth-last-child(3)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-last-child(3)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_last_child_collection() {
            let actual = Selectors::from_str("div:nth-last-child(0,3,8)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::NthLast(vec![0, 3, 8].into()),
            )]);

            assert_eq!(expected, actual, "element");

            let actual =
                Selectors::from_str("MyButton:nth-last-child(0,3,8)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::NthLast(vec![0, 3, 8].into()),
            )]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("MyButton:nth-last-child(0,3,,8)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-last-child(0,3,,8)'; value can not be blank"));

            assert_eq!(expected, actual, "blank index");

            let actual = Selectors::from_str("MyButton:nth-last-child(0,3,-8)");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-last-child(0,3,-8)'; invalid value '-8'",
            ));

            assert_eq!(expected, actual, "negative index");

            let actual = Selectors::from_str("MyButton:nth-last-child(zero,3,8)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-last-child(zero,3,8)'; invalid value 'zero'"));

            assert_eq!(expected, actual, "invalid index");

            let actual = Selectors::from_str("myButton:nth-last-child(0,3,8)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-last-child(0,3,8)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_child_range() {
            let actual = Selectors::from_str("div:nth-child(2..5)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::Range(Range { start: 2, end: 5 }),
            )]);

            assert_eq!(expected, actual, "not inclusive - element");

            let actual = Selectors::from_str("div:nth-child(2..=5)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::Range(Range { start: 2, end: 6 }),
            )]);

            assert_eq!(expected, actual, "inclusive - element");

            let actual = Selectors::from_str("MyButton:nth-child(2..5)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::Range(Range { start: 2, end: 5 }),
            )]);

            assert_eq!(expected, actual, "not inclusive - component");

            let actual = Selectors::from_str("MyButton:nth-child(2..=5)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::Range(Range { start: 2, end: 6 }),
            )]);

            assert_eq!(expected, actual, "inclusive - component");

            let actual = Selectors::from_str("MyButton:nth-child(5..2)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(5..2)'; range start cannot be more than end; 2 < 5"));

            assert_eq!(expected, actual, "start more than end");

            let actual = Selectors::from_str("MyButton:nth-child(5..=2)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(5..=2)'; range start cannot be more than inclusive end; =2 < 5"));

            assert_eq!(expected, actual, "start more than inclusive end");

            let actual = Selectors::from_str("MyButton:nth-child(-2..5)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(-2..5)'; '-2' is not a valid start value"));

            assert_eq!(expected, actual, "negative start");

            let actual = Selectors::from_str("MyButton:nth-child(two..5)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(two..5)'; 'two' is not a valid start value"));

            assert_eq!(expected, actual, "invalid start");

            let actual = Selectors::from_str("MyButton:nth-child(2..-5)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(2..-5)'; '-5' is not a valid end value"));

            assert_eq!(expected, actual, "negative end");

            let actual = Selectors::from_str("MyButton:nth-child(2..five)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(2..five)'; 'five' is not a valid end value"));

            assert_eq!(expected, actual, "invalid end");

            let actual = Selectors::from_str("MyButton:nth-child(2..=-5)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(2..=-5)'; '=-5' is not a valid end value"));

            assert_eq!(expected, actual, "negative inclusive end");

            let actual = Selectors::from_str("MyButton:nth-child(2..=five)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(2..=five)'; '=five' is not a valid end value"));

            assert_eq!(expected, actual, "invalid inclusive end");

            let actual = Selectors::from_str("myButton:nth-child(2..=5)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-child(2..=5)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_child_range_to() {
            let actual = Selectors::from_str("div:nth-child(..5)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::RangeTo(RangeTo { end: 5 }),
            )]);

            assert_eq!(expected, actual, "not inclusive - element");

            let actual = Selectors::from_str("div:nth-child(..=5)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::RangeTo(RangeTo { end: 6 }),
            )]);

            assert_eq!(expected, actual, "inclusive - element");

            let actual = Selectors::from_str("MyButton:nth-child(..5)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::RangeTo(RangeTo { end: 5 }),
            )]);

            assert_eq!(expected, actual, "not inclusive - component");

            let actual = Selectors::from_str("MyButton:nth-child(..=5)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::RangeTo(RangeTo { end: 6 }),
            )]);

            assert_eq!(expected, actual, "inclusive - component");

            let actual = Selectors::from_str("MyButton:nth-child(..-5)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(..-5)'; '-5' is not a valid end value"));

            assert_eq!(expected, actual, "negative not inclusive end");

            let actual = Selectors::from_str("MyButton:nth-child(..five)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(..five)'; 'five' is not a valid end value"));

            assert_eq!(expected, actual, "invalid not inclusive end");

            let actual = Selectors::from_str("MyButton:nth-child(..=-5)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(..=-5)'; '=-5' is not a valid end value"));

            assert_eq!(expected, actual, "negative inclusive end");

            let actual = Selectors::from_str("MyButton:nth-child(..=five)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(..=five)'; '=five' is not a valid end value"));

            assert_eq!(expected, actual, "invalid inclusive end");

            let actual = Selectors::from_str("myButton:nth-child(..5)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-child(..5)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_child_range_from() {
            let actual = Selectors::from_str("div:nth-child(2..)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::RangeFrom(RangeFrom { start: 2 }),
            )]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:nth-child(2..)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::RangeFrom(RangeFrom { start: 2 }),
            )]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("MyButton:nth-child(-2..)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(-2..)'; '-2' is not a valid start value"));

            assert_eq!(expected, actual, "negative start");

            let actual = Selectors::from_str("MyButton:nth-child(two..)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(two..)'; 'two' is not a valid start value"));

            assert_eq!(expected, actual, "invalid start");

            let actual = Selectors::from_str("myButton:nth-child(2..)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-child(2..)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_child_every_n() {
            let actual = Selectors::from_str("div:nth-child(3n)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::EveryN {
                    frequency: 3,
                    offset: 0,
                },
            )]);

            assert_eq!(expected, actual, "no offset - element");

            let actual = Selectors::from_str("div:nth-child(3n+2)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::EveryN {
                    frequency: 3,
                    offset: 2,
                },
            )]);

            assert_eq!(expected, actual, "positive offset - element");

            let actual = Selectors::from_str("div:nth-child(3n-2)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::Child,
                Nth::EveryN {
                    frequency: 3,
                    offset: -2,
                },
            )]);

            assert_eq!(expected, actual, "negative offset - element");

            let actual = Selectors::from_str("MyButton:nth-child(3n)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::EveryN {
                    frequency: 3,
                    offset: 0,
                },
            )]);

            assert_eq!(expected, actual, "no offset - component");

            let actual = Selectors::from_str("MyButton:nth-child(3n+2)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::EveryN {
                    frequency: 3,
                    offset: 2,
                },
            )]);

            assert_eq!(expected, actual, "positive offset - component");

            let actual = Selectors::from_str("MyButton:nth-child(3n-2)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::Child,
                Nth::EveryN {
                    frequency: 3,
                    offset: -2,
                },
            )]);

            assert_eq!(expected, actual, "negative offset - component");

            let actual = Selectors::from_str("MyButton:nth-child(-3n-2)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(-3n-2)'; '-3' is not a valid frequency value"));

            assert_eq!(expected, actual, "negative frequency");

            let actual = Selectors::from_str("MyButton:nth-child(threen-2)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(threen-2)'; 'three' is not a valid frequency value"));

            assert_eq!(expected, actual, "invalid frequency");

            let actual = Selectors::from_str("MyButton:nth-child(3ntwo)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-child(3ntwo)'; 'two' is not a valid offset value"));

            assert_eq!(expected, actual, "invalid offset");

            let actual = Selectors::from_str("myButton:nth-child(3n-2)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-child(3n-2)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_invalid_nth_of_type() {
            let actual = Selectors::from_str(":nth-of-type(even");
            let expected = Err(String::from("exception parsing selector ':nth-of-type(even'; '(even' is not a valid nth of type even expression"));

            assert_eq!(expected, actual, "incomplete even");

            let actual = Selectors::from_str(":nth-of-type(odd");
            let expected = Err(String::from("exception parsing selector ':nth-of-type(odd'; '(odd' is not a valid nth of type odd expression"));

            assert_eq!(expected, actual, "incomplete odd");

            let actual = Selectors::from_str(":nth-of-type(5");
            let expected = Err(String::from("exception parsing selector ':nth-of-type(5'; '(5' is not a valid nth of type expression"));

            assert_eq!(expected, actual, "incomplete nth");

            let actual = Selectors::from_str(":not(div");
            let expected = Err(String::from(
                "exception parsing selector ':not(div'; '(div' is not a valid not expression",
            ));

            assert_eq!(expected, actual, "incomplete not");

            let actual = Selectors::from_str(":nth-last-of-type(3");
            let expected = Err(String::from("exception parsing selector ':nth-last-of-type(3'; '(3' is not a valid nth last of type expression"));

            assert_eq!(expected, actual, "incomplete last");

            let actual = Selectors::from_str(":nth-of-type(..");
            let expected = Err(String::from("exception parsing selector ':nth-of-type(..'; Unnecessary/Invalid all '..' range selector, remove ':nth-of-type(..'"));

            assert_eq!(expected, actual, "unnecessary/invalid all");

            let actual = Selectors::from_str(":nth-of-type(2..5");
            let expected = Err(String::from("exception parsing selector ':nth-of-type(2..5'; '(2..5' is not a valid nth of type range expression"));

            assert_eq!(expected, actual, "incomplete range");

            let actual = Selectors::from_str(":nth-of-type(2..");
            let expected = Err(String::from("exception parsing selector ':nth-of-type(2..'; '(2..' is not a valid nth of type range from expression"));

            assert_eq!(expected, actual, "incomplete range from");

            let actual = Selectors::from_str(":nth-of-type(..5");
            let expected = Err(String::from("exception parsing selector ':nth-of-type(..5'; '(..5' is not a valid nth of type range to expression"));

            assert_eq!(expected, actual, "incomplete range to");

            let actual = Selectors::from_str(":nth-of-type(3n1");
            let expected = Err(String::from("exception parsing selector ':nth-of-type(3n1'; '(3n1' is not a valid nth every nth of type expression"));

            assert_eq!(expected, actual, "incomplete every n");

            let actual = Selectors::from_str(":nth-last-of-type(3n1");
            let expected = Err(String::from("exception parsing selector ':nth-last-of-type(3n1'; '(3n1' is not a valid nth last every nth of type expression"));

            assert_eq!(expected, actual, "incomplete last every n");
        }

        #[test]
        fn css_selector_nth_of_type_all() {
            let actual = Selectors::from_str("div:nth-of-type(..)");
            let expected = Err(String::from("exception parsing selector 'div:nth-of-type(..)'; Unnecessary 'all' range selector, remove ':nth-of-type(..)'"));

            assert_eq!(expected, actual, "_element");

            let actual = Selectors::from_str("MyButton:nth-of-type(..)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(..)'; Unnecessary 'all' range selector, remove ':nth-of-type(..)'"));

            assert_eq!(expected, actual, "_component");
        }

        #[test]
        fn css_selector_nth_of_type() {
            let actual = Selectors::from_str("div:nth-of-type(0)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::Nth(vec![0].into()),
            )]);

            assert_eq!(expected, actual, "first nth - element");

            let actual = Selectors::from_str("div:nth-of-type(3)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::Nth(vec![3].into()),
            )]);

            assert_eq!(expected, actual, "not first nth - element");

            let actual = Selectors::from_str("MyButton:nth-of-type(0)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::Nth(vec![0].into()),
            )]);

            assert_eq!(expected, actual, "first nth - component");

            let actual = Selectors::from_str("MyButton:nth-of-type(3)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::Nth(vec![3].into()),
            )]);

            assert_eq!(expected, actual, "not first nth - component");

            let actual = Selectors::from_str("MyButton:nth-of-type()");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-of-type()'; value can not be blank",
            ));

            assert_eq!(expected, actual, "blank index");

            let actual = Selectors::from_str("MyButton:nth-of-type(-3)");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-of-type(-3)'; invalid value '-3'",
            ));

            assert_eq!(expected, actual, "negative index");

            let actual = Selectors::from_str("MyButton:nth-of-type(three)");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-of-type(three)'; invalid value 'three'",
            ));

            assert_eq!(expected, actual, "invalid index");

            let actual = Selectors::from_str("myButton:nth-of-type(3)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-of-type(3)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_of_type_collection() {
            let actual = Selectors::from_str("div:nth-of-type(0,3,8)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::Nth(vec![0, 3, 8].into()),
            )]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:nth-of-type(0,3,8)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::Nth(vec![0, 3, 8].into()),
            )]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("MyButton:nth-of-type(0,3,,8)");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-of-type(0,3,,8)'; value can not be blank",
            ));

            assert_eq!(expected, actual, "blank index");

            let actual = Selectors::from_str("MyButton:nth-of-type(0,3,-8)");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-of-type(0,3,-8)'; invalid value '-8'",
            ));

            assert_eq!(expected, actual, "negative index");

            let actual = Selectors::from_str("MyButton:nth-of-type(zero,3,8)");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-of-type(zero,3,8)'; invalid value 'zero'",
            ));

            assert_eq!(expected, actual, "invalid index");

            let actual = Selectors::from_str("myButton:nth-of-type(0,3,8)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-of-type(0,3,8)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_of_type_even() {
            let actual = Selectors::from_str("div:nth-of-type(even)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::TypeOf, Nth::Even)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:nth-of-type(even)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::TypeOf, Nth::Even)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton:nth-of-type(even)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-of-type(even)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_of_type_odd() {
            let actual = Selectors::from_str("div:nth-of-type(odd)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::TypeOf, Nth::Odd)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:nth-of-type(odd)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::TypeOf, Nth::Odd)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton:nth-of-type(odd)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-of-type(odd)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_of_type_first() {
            let actual = Selectors::from_str("div:first-of-type").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::TypeOf, Nth::First)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:first-of-type").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::TypeOf, Nth::First)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton:first-of-type");
            let expected = Err(String::from("exception parsing selector 'myButton:first-of-type'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_of_type_last() {
            let actual = Selectors::from_str("div:last-of-type").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("div", SelectorMode::TypeOf, Nth::Last)]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:last-of-type").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![("MyButton", SelectorMode::TypeOf, Nth::Last)]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("myButton:last-of-type");
            let expected = Err(String::from("exception parsing selector 'myButton:last-of-type'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_last_of_type() {
            let actual = Selectors::from_str("div:nth-last-of-type(0)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::NthLast(vec![0].into()),
            )]);

            assert_eq!(expected, actual, "last nth - element");

            let actual = Selectors::from_str("div:nth-last-of-type(3)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::NthLast(vec![3].into()),
            )]);

            assert_eq!(expected, actual, "not last nth - element");

            let actual =
                Selectors::from_str("MyButton:nth-last-of-type(0)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::NthLast(vec![0].into()),
            )]);

            assert_eq!(expected, actual, "last nth - component");

            let actual =
                Selectors::from_str("MyButton:nth-last-of-type(3)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::NthLast(vec![3].into()),
            )]);

            assert_eq!(expected, actual, "not last nth - component");

            let actual = Selectors::from_str("MyButton:nth-last-of-type()");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-last-of-type()'; value can not be blank",
            ));

            assert_eq!(expected, actual, "blank index");

            let actual = Selectors::from_str("MyButton:nth-last-of-type(-3)");
            let expected = Err(String::from(
                "exception parsing selector 'MyButton:nth-last-of-type(-3)'; invalid value '-3'",
            ));

            assert_eq!(expected, actual, "negative index");

            let actual = Selectors::from_str("MyButton:nth-last-of-type(three)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-last-of-type(three)'; invalid value 'three'"));

            assert_eq!(expected, actual, "invalid index");

            let actual = Selectors::from_str("myButton:nth-last-of-type(3)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-last-of-type(3)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_last_of_type_collection() {
            let actual = Selectors::from_str("div:nth-last-of-type(0,3,8)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::NthLast(vec![0, 3, 8].into()),
            )]);

            assert_eq!(expected, actual, "element");

            let actual =
                Selectors::from_str("MyButton:nth-last-of-type(0,3,8)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::NthLast(vec![0, 3, 8].into()),
            )]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("MyButton:nth-last-of-type(0,3,,8)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-last-of-type(0,3,,8)'; value can not be blank"));

            assert_eq!(expected, actual, "blank index");

            let actual = Selectors::from_str("MyButton:nth-last-of-type(0,3,-8)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-last-of-type(0,3,-8)'; invalid value '-8'"));

            assert_eq!(expected, actual, "negative index");

            let actual = Selectors::from_str("MyButton:nth-last-of-type(zero,3,8)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-last-of-type(zero,3,8)'; invalid value 'zero'"));

            assert_eq!(expected, actual, "invalid index");

            let actual = Selectors::from_str("myButton:nth-last-of-type(0,3,8)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-last-of-type(0,3,8)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_of_type_range() {
            let actual = Selectors::from_str("div:nth-of-type(2..5)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::Range(Range { start: 2, end: 5 }),
            )]);

            assert_eq!(expected, actual, "not inclusive - element");

            let actual = Selectors::from_str("div:nth-of-type(2..=5)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::Range(Range { start: 2, end: 6 }),
            )]);

            assert_eq!(expected, actual, "inclusive - element");

            let actual = Selectors::from_str("MyButton:nth-of-type(2..5)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::Range(Range { start: 2, end: 5 }),
            )]);

            assert_eq!(expected, actual, "not inclusive - component");

            let actual = Selectors::from_str("MyButton:nth-of-type(2..=5)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::Range(Range { start: 2, end: 6 }),
            )]);

            assert_eq!(expected, actual, "inclusive - component");

            let actual = Selectors::from_str("MyButton:nth-of-type(5..2)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(5..2)'; range start cannot be more than end; 2 < 5"));

            assert_eq!(expected, actual, "start more than end");

            let actual = Selectors::from_str("MyButton:nth-of-type(5..=2)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(5..=2)'; range start cannot be more than inclusive end; =2 < 5"));

            assert_eq!(expected, actual, "start more than inclusive end");

            let actual = Selectors::from_str("MyButton:nth-of-type(-2..5)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(-2..5)'; '-2' is not a valid start value"));

            assert_eq!(expected, actual, "negative start");

            let actual = Selectors::from_str("MyButton:nth-of-type(two..5)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(two..5)'; 'two' is not a valid start value"));

            assert_eq!(expected, actual, "invalid start");

            let actual = Selectors::from_str("MyButton:nth-of-type(2..-5)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(2..-5)'; '-5' is not a valid end value"));

            assert_eq!(expected, actual, "negative end");

            let actual = Selectors::from_str("MyButton:nth-of-type(2..five)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(2..five)'; 'five' is not a valid end value"));

            assert_eq!(expected, actual, "invalid end");

            let actual = Selectors::from_str("MyButton:nth-of-type(2..=-5)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(2..=-5)'; '=-5' is not a valid end value"));

            assert_eq!(expected, actual, "negative inclusive end");

            let actual = Selectors::from_str("MyButton:nth-of-type(2..=five)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(2..=five)'; '=five' is not a valid end value"));

            assert_eq!(expected, actual, "invalid inclusive end");

            let actual = Selectors::from_str("myButton:nth-of-type(2..5)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-of-type(2..5)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_of_type_range_to() {
            let actual = Selectors::from_str("div:nth-of-type(..5)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::RangeTo(RangeTo { end: 5 }),
            )]);

            assert_eq!(expected, actual, "not inclusive - element");

            let actual = Selectors::from_str("div:nth-of-type(..=5)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::RangeTo(RangeTo { end: 6 }),
            )]);

            assert_eq!(expected, actual, "inclusive - element");

            let actual = Selectors::from_str("MyButton:nth-of-type(..5)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::RangeTo(RangeTo { end: 5 }),
            )]);

            assert_eq!(expected, actual, "not inclusive - component");

            let actual = Selectors::from_str("MyButton:nth-of-type(..=5)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::RangeTo(RangeTo { end: 6 }),
            )]);

            assert_eq!(expected, actual, "inclusive - component");

            let actual = Selectors::from_str("MyButton:nth-of-type(..-5)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(..-5)'; '-5' is not a valid end value"));

            assert_eq!(expected, actual, "negative not inclusive end");

            let actual = Selectors::from_str("MyButton:nth-of-type(..five)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(..five)'; 'five' is not a valid end value"));

            assert_eq!(expected, actual, "invalid not inclusive end");

            let actual = Selectors::from_str("MyButton:nth-of-type(..=-5)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(..=-5)'; '=-5' is not a valid end value"));

            assert_eq!(expected, actual, "negative inclusive end");

            let actual = Selectors::from_str("MyButton:nth-of-type(..=five)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(..=five)'; '=five' is not a valid end value"));

            assert_eq!(expected, actual, "invalid inclusive end");

            let actual = Selectors::from_str("myButton:nth-of-type(..5)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-of-type(..5)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_of_type_range_from() {
            let actual = Selectors::from_str("div:nth-of-type(2..)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::RangeFrom(RangeFrom { start: 2 }),
            )]);

            assert_eq!(expected, actual, "element");

            let actual = Selectors::from_str("MyButton:nth-of-type(2..)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::RangeFrom(RangeFrom { start: 2 }),
            )]);

            assert_eq!(expected, actual, "component");

            let actual = Selectors::from_str("MyButton:nth-of-type(-2..)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(-2..)'; '-2' is not a valid start value"));

            assert_eq!(expected, actual, "negative start");

            let actual = Selectors::from_str("MyButton:nth-of-type(two..)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(two..)'; 'two' is not a valid start value"));

            assert_eq!(expected, actual, "invalid start");

            let actual = Selectors::from_str("myButton:nth-of-type(2..)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-of-type(2..)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }

        #[test]
        fn css_selector_nth_of_type_every_n() {
            let actual = Selectors::from_str("div:nth-of-type(3n)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::EveryN {
                    frequency: 3,
                    offset: 0,
                },
            )]);

            assert_eq!(expected, actual, "no offset - element");

            let actual = Selectors::from_str("div:nth-of-type(3n+2)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::EveryN {
                    frequency: 3,
                    offset: 2,
                },
            )]);

            assert_eq!(expected, actual, "positive offset - element");

            let actual = Selectors::from_str("div:nth-of-type(3n-2)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "div",
                SelectorMode::TypeOf,
                Nth::EveryN {
                    frequency: 3,
                    offset: -2,
                },
            )]);

            assert_eq!(expected, actual, "negative offset - element");

            let actual = Selectors::from_str("MyButton:nth-of-type(3n)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::EveryN {
                    frequency: 3,
                    offset: 0,
                },
            )]);

            assert_eq!(expected, actual, "no offset - component");

            let actual = Selectors::from_str("MyButton:nth-of-type(3n+2)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::EveryN {
                    frequency: 3,
                    offset: 2,
                },
            )]);

            assert_eq!(expected, actual, "positive offset - component");

            let actual = Selectors::from_str("MyButton:nth-of-type(3n-2)").expect(VALID_SELECTORS);
            let expected = expected_selectors(vec![(
                "MyButton",
                SelectorMode::TypeOf,
                Nth::EveryN {
                    frequency: 3,
                    offset: -2,
                },
            )]);

            assert_eq!(expected, actual, "negative offset - component");

            let actual = Selectors::from_str("MyButton:nth-of-type(-3n-2)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(-3n-2)'; '-3' is not a valid frequency value"));

            assert_eq!(expected, actual, "negative frequency");

            let actual = Selectors::from_str("MyButton:nth-of-type(threen-2)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(threen-2)'; 'three' is not a valid frequency value"));

            assert_eq!(expected, actual, "invalid frequency");

            let actual = Selectors::from_str("MyButton:nth-of-type(3ntwo)");
            let expected = Err(String::from("exception parsing selector 'MyButton:nth-of-type(3ntwo)'; 'two' is not a valid offset value"));

            assert_eq!(expected, actual, "invalid offset");

            let actual = Selectors::from_str("myButton:nth-of-type(3n-2)");
            let expected = Err(String::from("exception parsing selector 'myButton:nth-of-type(3n-2)'; 'myButton' is an invalid component name"));

            assert_eq!(expected, actual, "invalid element");
        }
    }

    mod selector_matching {
        use utils::test_selector_matching;

        #[test]
        fn select_all_simple() {
            // select all div in root level
            let sut = "div";
            let dom_tests = "
                div         ✔ first match
                MyButton    ✘ MyButton
                div
                    div     ✘ div 2nd level
                div         ✔ subsequent match
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_all_multi_level() {
            // select all 2nd level MyButton in any div in root level
            let sut = "div > MyButton";
            let dom_tests = "
                div
                    MyButton        ✔ first match
                    OtherButton     ✘ OtherButton
                    MyButton        ✔ subsequent match
                div
                    div
                    MyButton        ✔ 2nd branch
                div
                    div
                        MyButton    ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_not_element() {
            // select all 3rd level div in 2nd level elements not MyLabel or input in any div in root level
            let sut = "div > :not(MyLabel,input) > div";
            let dom_tests = "
                div
                    input
                        div         ✘ input
                    div
                        div         ✔ first match
                    p
                        div         ✔ subsequent match
                div
                    button
                        div         ✔ 2nd branch
                            div     ✘ deep child
                    MyLabel
                        div         ✘ MyLabel
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_first_child() {
            // select all 3rd level div in 2nd level first div in any div in root level
            let sut = "div > div:first-child > div";
            let dom_tests = "
                div
                    div
                        div         ✔ first match
                        label       ✘ label
                        div         ✔ subsequent match
                    div
                        div         ✘ 2nd div
                div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_last_child() {
            // select all 3rd level div in 2nd level last div in any div in root level
            let sut = "div > div:last-child > div";
            let dom_tests = "
                div
                    div
                        div         ✘ 1st div
                        label       ✘ label
                        div         ✘ 2nd div
                    div
                        div         ✔ first match
                div
                    div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_only_child() {
            // select all 3rd level div in 2nd level only child div in any root div
            let sut = "div > div:only-child > div";
            let dom_tests = "
                div
                    div
                        div         ✔ first match
                div
                    div
                    div
                        div         ✘ not only child
                div
                    label
                        div         ✘ not only div child
                div
                    div
                        div         ✔ 2nd branch
                div
                    div
                        div
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_child_even() {
            // select all 3rd level div in 2nd level even div in any root div
            let sut = "div > div:nth-child(even) > div";
            let dom_tests = "
                div
                    label           ✘ 2nd level label
                    div
                        div         ✔ first match
                        label       ✘ 3rd level label
                        div         ✔ subsequent match
                    div
                        div         ✘ 2nd odd branch
                    div
                        div         ✔ 3rd even branch
                div
                    label
                    div
                        div         ✔ 2nd root branch
                            div     ✘ deep div
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_child_odd() {
            // select all 3rd level div in 2nd level odd div in any root div
            let sut = "div > div:nth-child(odd) > div";
            let dom_tests = "
                div
                    div
                        div         ✔ first match
                        label       ✘ label
                    div
                        div         ✘ even level div
                    label
                    label
                    div
                        div         ✔ subsequent match
                    div
                        div         ✘ even level div
                div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_child_every_n_no_offset() {
            // select all 3rd level div in 2nd level every 3rd div in any root div
            let sut = "div > div:nth-child(3n) > div";
            let dom_tests = "
                div
                    div
                        div         ✘ 1st - div
                        label       ✘ 1st - label
                    label
                        div         ✘ 2nd - div
                    div
                        div         ✔ first match
                    label
                    div
                        div         ✘ 5th - div
                    div
                        div         ✔ subsequent match
                div
                    div
                    div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_child_every_n_positive_offset() {
            // select all 3rd level div in 2nd level every 3rd div offset by 4 in any root div
            let sut = "div > div:nth-child(3n+7) > div";
            let dom_tests = "
                div
                    div
                        div         ✔ first match
                        label       ✘ 1st - label
                    label
                        div         ✘ 2nd - div
                    div
                        div         ✘ 3rd - div
                    div
                        div         ✔ subsequent match
                    label
                    div
                        div         ✘ 5th - div
                div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_child_every_n_negative_offset() {
            // select all 3rd level div in 2nd level every 3rd div offset by -2 in any root div
            let sut = "div > div:nth-child(3n-8) > div";
            let dom_tests = "
                div
                    div
                        div         ✔ first match
                        label       ✘ 1st - label
                    label
                        div         ✘ 2nd - div
                    div
                        div         ✘ 3rd - div
                    div
                        div         ✔ subsequent match
                    label
                    div
                        div         ✘ 5th - div
                div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_last_child_every_n_no_offset() {
            // select all 3rd level div in 2nd level every last 3rd div in any root div
            let sut = "div > div:nth-last-child(3n) > div";
            let dom_tests = "
                div
                    div
                        div         ✔ first match
                        label       ✘ 1st - label
                    label
                        div         ✘ 2nd - div
                    div
                        div         ✘ 3rd - div
                    div
                        div         ✔ subsequent match
                    label
                    div
                        div         ✘ 5th - div
                div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
                    div
                    div
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_last_child_every_n_positive_offset() {
            // select all 3rd level div in 2nd level every last 3rd div offset by 4 in any root div
            let sut = "div > div:nth-last-child(3n+7) > div";
            let dom_tests = "
                div
                    div
                        div         ✘ 1st - div
                        label       ✘ 1st - label
                    label
                        div         ✘ 2nd - div
                    div
                        div         ✔ first match
                    div
                        div         ✘ 4th - div
                    label
                    div
                        div         ✔ subsequent match
                div
                    div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
                    div
                    label
                    div
                        div
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_last_child_every_n_negative_offset() {
            // select all 3rd level div in 2nd level every last 3rd div offset by -2 in any root div
            let sut = "div > div:nth-last-child(3n-8) > div";
            let dom_tests = "
                div
                    div
                        div         ✘ 1st - div
                        label       ✘ 1st - label
                    label
                        div         ✘ 2nd - div
                    div
                        div         ✔ first match
                    div
                        div         ✘ 4th - div
                    label
                    div
                        div         ✔ subsequent match
                div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
                    div
                        div
                    label
                        div
                    div
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_child_range() {
            // select all 3rd level div in 2nd level div from 2nd through 4th in any root div
            let sut = "div > div:nth-child(2..5) > div";
            let dom_tests = "
                div
                    div
                        div         ✘ 1st - div
                        label       ✘ 1st - label
                    div
                        div         ✔ first match
                    label
                        div         ✘ 2nd - div
                    div
                        div         ✔ subsequent match
                    label
                        div         ✘ 5th - div
                div
                    div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_child_range_inclusive() {
            // select all 3rd level div in 2nd level div from 2nd through 4th in any root div
            let sut = "div > div:nth-child(2..=4) > div";
            let dom_tests = "
                div
                    div
                        div         ✘ 1st - div
                        label       ✘ 1st - label
                    div
                        div         ✔ first match
                    label
                        div         ✘ 2nd - div
                    div
                        div         ✔ subsequent match
                    label
                        div         ✘ 5th - div
                div
                    div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_child_range_from() {
            // select all 3rd level div in 2nd level div from 2nd through last in any root div
            let sut = "div > div:nth-child(2..) > div";
            let dom_tests = "
                div
                    div
                        div         ✘ 1st - div
                        label       ✘ 1st - label
                    div
                        div         ✔ first match
                    label
                        div         ✘ 2nd - div
                    div
                        div         ✔ subsequent match
                    label
                        div         ✘ 5th - div
                    div
                        div         ✔ another match
                div
                    div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_child_range_to() {
            // select all 3rd level div in 2nd level div from 1st through 4th in any root div
            let sut = "div > div:nth-child(..5) > div";
            let dom_tests = "
                div
                    div
                        div         ✔ first match
                        label       ✘ 1st - label
                    div
                        div         ✔ subsequent match
                    label
                        div         ✘ 2nd - div
                    div
                        div         ✔ another match
                    label
                        div         ✘ 5th - div
                div
                    div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_child_range_to_inclusive() {
            // select all 3rd level div in 2nd level div from 1st through 4th in any root div
            let sut = "div > div:nth-child(..=4) > div";
            let dom_tests = "
                div
                    div
                        div         ✔ first match
                        label       ✘ 1st - label
                    div
                        div         ✔ subsequent match
                    label
                        div         ✘ 2nd - div
                    div
                        div         ✔ another match
                    label
                        div         ✘ 5th - div
                div
                    div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_child() {
            // select all 3rd level div in 2nd level 2nd and 5th div in any root div
            let sut = "div > div:nth-child(2,5) > div";
            let dom_tests = "
                div
                    div
                        div         ✘ 1st - div
                        label       ✘ 1st - label
                    div
                        div         ✔ first match
                    label
                        div         ✘ 2nd - div
                    label
                    div
                        div         ✔ subsequent match
                div
                    div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_last_child() {
            // select all 3rd level div in 2nd level last 2nd and 5th div in any root div
            let sut = "div > div:nth-last-child(2,5) > div";
            let dom_tests = "
                div
                    div
                    div
                        div         ✔ first match
                    div
                        div         ✘ 2nd - div
                        label       ✘ 2nd - label
                    label
                        div         ✘ 3rd - div
                    div
                        div         ✔ subsequent match
                    label
                div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
                    div
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_of_type_even() {
            // select all 3rd level div in 2nd level even div in any root div
            let sut = "div > div:nth-of-type(even) > div";
            let dom_tests = "
                div
                    label           ✘ 2nd level label
                    div
                    div
                        div         ✔ first match
                        label       ✘ 3rd level label
                        div         ✔ subsequent match
                    div
                        div         ✘ odd branch
                    div
                        div         ✔ another match
                div
                    div
                        label
                    div
                        div         ✔ 2nd root branch
                            div     ✘ deep div
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_of_type_odd() {
            // select all 3rd level div in 2nd level odd div in any root div
            let sut = "div > div:nth-of-type(odd) > div";
            let dom_tests = "
                div
                    div
                        div         ✔ first match
                        label       ✘ label
                    div
                        div         ✘ even level div
                    label
                    div
                        div         ✔ subsequent match
                    div
                        div         ✘ even level div
                div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_of_type_every_n_no_offset() {
            // select all 3rd level div in 2nd level every 3rd div in any root div
            let sut = "div > div:nth-of-type(3n) > div";
            let dom_tests = "
                div
                    div
                        div         ✘ 1st - div
                        label       ✘ 1st - label
                    div
                        div         ✘ 2nd - div
                    div
                        div         ✔ first match
                    label
                    div
                        div         ✘ 5th - div
                    label
                        div         ✘ 6th - div
                    div
                    div
                        div         ✔ subsequent match
                div
                    div
                        label
                    div
                    label
                        div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_of_type_every_n_positive_offset() {
            // select all 3rd level div in 2nd level every 3rd div offset by 4 in any root div
            let sut = "div > div:nth-of-type(3n+7) > div";
            let dom_tests = "
                div
                    div
                        div         ✔ first match
                        label       ✘ 1st - label
                    label
                        div         ✘ 2nd - div
                    div
                    div
                        div         ✘ 4th - div
                    div
                        div         ✔ subsequent match
                    label
                        div
                    div
                    div
                        div         ✘ 8th - div
                div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
                    label
                        div
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_of_type_every_n_negative_offset() {
            // select all 3rd level div in 2nd level every 3rd div offset by -2 in any root div
            let sut = "div > div:nth-of-type(3n-8) > div";
            let dom_tests = "
                div
                    div
                        div         ✔ first match
                        label       ✘ 1st - label
                    label
                        div         ✘ 2nd - div
                    div
                    div
                        div         ✘ 3rd - div
                    div
                        div         ✔ subsequent match
                    label
                    div
                    div
                        div         ✘ 5th - div
                div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_last_of_type_every_n_no_offset() {
            // select all 3rd level div in 2nd level every last 3rd div in any root div
            let sut = "div > div:nth-last-of-type(3n) > div";
            let dom_tests = "
                div
                    div
                        div         ✔ first match
                        label       ✘ 1st - label
                    label
                        div         ✘ 2nd - div
                    div
                    div
                        div         ✘ 4th - div
                    div
                        div         ✔ subsequent match
                    label
                    div
                        div         ✘ 7th - div
                    input
                        label
                    div
                        div         ✘ 9th - div
                div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
                    div
                    label
                        div
                    div
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_last_of_type_every_n_positive_offset() {
            // select all 3rd level div in 2nd level every last 3rd div offset by 4 in any root div
            let sut = "div > div:nth-last-of-type(3n+7) > div";
            let dom_tests = "
                div
                    div
                        div         ✘ 1st - div
                        label       ✘ 1st - label
                    label
                        div         ✘ 2nd - div
                    div
                    div
                        div         ✔ first match
                    label
                    div
                        div         ✘ 6th - div
                    label
                    div
                    div
                        div         ✔ subsequent match
                    label
                div
                    div
                        input
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_last_of_type_every_n_negative_offset() {
            // select all 3rd level div in 2nd level every last 3rd div offset by -2 in any root div
            let sut = "div > div:nth-last-of-type(3n-8) > div";
            let dom_tests = "
                div
                    div
                        div         ✔ first match
                        label       ✘ 1st - label
                    label
                        div         ✘ 2nd - div
                    div
                        div         ✘ 3rd div
                    div
                        div         ✘ 4th - div
                    label
                    div
                        div         ✔ subsequent match
                    label
                div
                    div
                        div
                    label
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
                        div
                    label
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_of_type_range() {
            // select all 3rd level div in 2nd level div from 2nd through 4th in any root div
            let sut = "div > div:nth-of-type(2..5) > div";
            let dom_tests = "
                div
                    label
                        div         ✘ 1st - div
                    div
                        div         ✘ 2nd - 1st div - div
                        label       ✘ 2nd - 1st div - label
                    div
                        div         ✔ first match
                    label
                        div         ✘ 4th - div
                    div
                        div         ✔ subsequent match
                    label
                        div         ✘ 6th - div
                div
                    div
                    label
                        div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_of_type_range_inclusive() {
            // select all 3rd level div in 2nd level div from 2nd through 4th in any root div
            let sut = "div > div:nth-of-type(2..=4) > div";
            let dom_tests = "
                div
                    label
                        div         ✘ 1st - div
                    div
                        div         ✘ 2nd - 1st div - div
                        label       ✘ 2nd - 1st div - label
                    div
                        div         ✔ first match
                    label
                        div         ✘ 4th - div
                    div
                        div         ✔ subsequent match
                    label
                        div         ✘ 6th - div
                div
                    div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_of_type_range_from() {
            // select all 3rd level div in 2nd level div from 2nd through last in any root div
            let sut = "div > div:nth-of-type(2..) > div";
            let dom_tests = "
                div
                    label
                        div         ✘ 1st - div
                    div
                        div         ✘ 2nd - 1st div - div
                        label       ✘ 2nd - 1st div - label
                    div
                        div         ✔ first match
                    label
                        div         ✘ 4th - div
                    div
                        div         ✔ subsequent match
                    label
                        div         ✘ 6th - div
                    div
                        div         ✔ another match
                div
                    div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_of_type_range_to() {
            // select all 3rd level div in 2nd level div from 1st through 4th in any root div
            let sut = "div > div:nth-of-type(..5) > div";
            let dom_tests = "
                div
                    label
                        div         ✘ 1st - div
                    div
                        div         ✔ first match
                        label       ✘ 2nd - label
                    div
                        div         ✔ subsequent match
                    label
                        div         ✘ 4th - div
                    div
                        div         ✔ another match
                    label
                        div         ✘ 5th - div
                div
                    div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_of_type_range_to_inclusive() {
            // select all 3rd level div in 2nd level div from 1st through 4th in any root div
            let sut = "div > div:nth-of-type(..=4) > div";
            let dom_tests = "
                div
                    label
                        div         ✘ 1st - div
                    div
                        div         ✔ first match
                        label       ✘ 2nd - label
                    div
                        div         ✔ subsequent match
                    label
                        div         ✘ 4th - div
                    div
                        div         ✔ another match
                    label
                        div         ✘ 5th - div
                div
                    div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_of_type() {
            // select all 3rd level div in 2nd level 2nd and 5th div in any root div
            let sut = "div > div:nth-of-type(2,5) > div";
            let dom_tests = "
                div
                    div
                        div         ✘ 1st - div
                        label       ✘ 1st - label
                    div
                        div         ✔ first match
                    div
                        div         ✘ 2nd - div
                    div
                    div
                        div         ✔ subsequent match
                div
                    div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
            ";

            test_selector_matching(sut, dom_tests);
        }

        #[test]
        fn select_nth_last_of_type() {
            // select all 3rd level div in 2nd level last 2nd and 5th div in any root div
            let sut = "div > div:nth-last-of-type(2,5) > div";
            let dom_tests = "
                div
                    div
                    div
                        div         ✔ first match
                    div
                        div         ✘ 3rd - div
                        label       ✘ 3rd - label
                    label
                        div         ✘ 4th - div
                    div
                    div
                        div         ✔ subsequent match
                    div
                    label
                        div
                div
                    div
                        div         ✔ 2nd branch
                            div     ✘ deep child
                    label
                        div
                    div
                    label
            ";

            test_selector_matching(sut, dom_tests);
        }

        mod utils {
            use std::cell::RefCell;
            use std::collections::{HashMap, VecDeque};
            use std::ops::{Deref, DerefMut};
            use std::rc::Rc;
            use std::str::FromStr;

            use crate::props::injection::{Branch, Selectors};

            use super::super::VALID_SELECTORS;

            const SIBLING_ADDED: &str = "sibling added";
            const BRANCH_FINISHED: &str = "branch finished";
            const BRANCH_TOTAL: &str = "branch total";

            /// defines a node of a testing dom
            #[cfg_attr(debug_assertions, derive(Debug))]
            enum Node<'a> {
                /// creates a new child branch level, first one is root
                Child(&'a str),

                /// adds next child
                Sibling(&'a str),

                /// completes current child branch, pops up one level
                Finish,
            }

            /// defines test expected
            #[cfg_attr(debug_assertions, derive(Debug))]
            enum Expected<'a> {
                /// Nth matches
                Matches(&'a str),

                /// Nth does not match
                NotMatches(&'a str),

                /// End of Test Iterator
                EndOfTests,
            }

            /// Text representation of Tested DOM with test branches
            #[cfg_attr(debug_assertions, derive(Debug))]
            struct DomTestBranches<'a> {
                /// individual test lines
                dom: VecDeque<&'a str>,

                /// leading spaces to remove, caused by using an indented string literal
                leading_spaces: usize,
            }

            impl<'a> DomTestBranches<'a> {
                fn new(src_dom: &'a str) -> Self {
                    // split source by cr, filter out empty lines
                    let dom = src_dom
                        .split('\n')
                        .filter_map(|line| {
                            let line = line.trim_end();

                            if line.is_empty() {
                                None
                            } else {
                                Some(line)
                            }
                        })
                        .collect::<VecDeque<_>>();

                    // calc leading spaces to remove from first line
                    let leading_spaces = dom
                        .get(0)
                        .map(|line| line.len() - line.trim().len())
                        .unwrap_or_default();

                    Self {
                        dom,
                        leading_spaces,
                    }
                }
            }

            impl<'a> Deref for DomTestBranches<'a> {
                type Target = VecDeque<&'a str>;

                fn deref(&self) -> &Self::Target {
                    &self.dom
                }
            }

            impl<'a> DerefMut for DomTestBranches<'a> {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.dom
                }
            }

            impl<'a> From<DomTestBranches<'a>> for Vec<(Vec<Node<'a>>, Expected<'a>)> {
                fn from(dom: DomTestBranches<'a>) -> Self {
                    dom.into_iter().collect()
                }
            }

            /// All branch totals for a given test dom
            /// Branch uses total for checking dom tree matching
            #[cfg_attr(debug_assertions, derive(Debug))]
            struct BranchTotals(Vec<(usize, Rc<HashMap<String, usize>>)>);

            impl<'a> FromIterator<&'a Node<'a>> for BranchTotals {
                fn from_iter<T: IntoIterator<Item = &'a Node<'a>>>(source: T) -> Self {
                    const MORE_TOTALS: &str = "expected more parent totals";

                    // calculated totals
                    let mut totals = VecDeque::new();
                    // stack of of type totals for parent branches
                    let mut stack = Vec::new();
                    // count by type collector
                    let mut of_type = Rc::new(RefCell::new(HashMap::new()));

                    for test_branch in source.into_iter() {
                        match test_branch {
                            Node::Child(element) => {
                                stack.push(of_type.clone());

                                of_type = Rc::new(RefCell::new(HashMap::new()));

                                // branch only needs the total of a branch,
                                // determined at new child
                                totals.push_front(of_type.clone());

                                update_element_count(element, &of_type);
                            }
                            Node::Sibling(element) => update_element_count(element, &of_type),
                            Node::Finish => of_type = stack.pop().expect(MORE_TOTALS),
                        }
                    }

                    let totals = totals
                        .into_iter()
                        .map(|itm| {
                            let of_type = Rc::new(itm.take());
                            let total: usize = of_type.values().sum();

                            (total, of_type)
                        })
                        .collect();

                    return Self(totals);

                    #[inline]
                    fn update_element_count(
                        element: &str,
                        of_type: &Rc<RefCell<HashMap<String, usize>>>,
                    ) {
                        {
                            // update current branch level's count for element type
                            let mut of_type = of_type.borrow_mut();
                            let entry = of_type.entry(element.to_string()).or_insert(0);

                            *entry += 1;
                        }
                    }
                }
            }

            impl Deref for BranchTotals {
                type Target = Vec<(usize, Rc<HashMap<String, usize>>)>;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl DerefMut for BranchTotals {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }

            impl<'a> IntoIterator for DomTestBranches<'a> {
                type Item = (Vec<Node<'a>>, Expected<'a>);
                type IntoIter = TestsIterator<'a>;

                fn into_iter(self) -> Self::IntoIter {
                    TestsIterator {
                        dom: self,
                        current_depth: -1,
                        nodes: Vec::new(),
                    }
                }
            }

            /// Iterates the tests of DOM
            struct TestsIterator<'a> {
                /// dom tests to iterator
                dom: DomTestBranches<'a>,

                /// tracks the current depth into the dom tree
                current_depth: isize,

                /// tracks the nodes from a dom tree, for a test
                nodes: Vec<Node<'a>>,
            }

            impl<'a> Iterator for TestsIterator<'a> {
                type Item = (Vec<Node<'a>>, Expected<'a>);

                fn next(&mut self) -> Option<Self::Item> {
                    const INDENTATION: isize = 4;

                    let indentation = self.dom.leading_spaces;
                    let dom_tests = &mut self.dom;

                    loop {
                        let element_test = match dom_tests.pop_front() {
                            Some(element_test) => element_test,
                            None if !self.nodes.is_empty() => {
                                let branches = std::mem::replace(&mut self.nodes, Vec::new());

                                return Some((branches, Expected::EndOfTests));
                            }
                            None => return None,
                        };
                        let element_test = element_test[indentation..].trim_end();
                        let spacers =
                            element_test.chars().take_while(|chr| *chr == ' ').count() as isize;
                        let depth = spacers / INDENTATION;

                        if depth < self.current_depth {
                            while self.current_depth != depth {
                                self.nodes.push(Node::Finish);
                                self.current_depth -= 1;
                            }
                        }

                        let mut branch = |elm, depth| {
                            if depth == self.current_depth {
                                Node::Sibling(elm)
                            } else {
                                self.current_depth += 1;
                                Node::Child(elm)
                            }
                        };

                        if let Some((element, test)) = element_test.split_once('✔') {
                            let element = element.trim();
                            let test = Expected::Matches(test.trim());

                            self.nodes.push(branch(element, depth));

                            let branches = std::mem::replace(&mut self.nodes, Vec::new());

                            return Some((branches, test));
                        }

                        if let Some((element, test)) = element_test.split_once('✘') {
                            let element = element.trim();
                            let test = Expected::NotMatches(test.trim());

                            self.nodes.push(branch(element, depth));

                            let branches = std::mem::replace(&mut self.nodes, Vec::new());

                            return Some((branches, test));
                        }

                        let element = element_test.trim();

                        self.nodes.push(branch(element, depth));
                    }
                }
            }

            pub fn test_selector_matching(
                // selectors being tested
                selectors: &str,
                // dom selector match tests definition
                dom_tests: &str,
            ) {
                let sut = Selectors::from_str(selectors).expect(VALID_SELECTORS);
                let mut branch = Branch::new();
                let dom_tests: Vec<_> = DomTestBranches::new(dom_tests).into();
                let mut totals = dom_tests
                    .iter()
                    .flat_map(|(steps, _)| steps)
                    .collect::<BranchTotals>();

                for (branches, expected) in &dom_tests {
                    //
                    for test_branch in branches {
                        // build trace branch used to test css matching
                        match test_branch {
                            Node::Child(tag) => {
                                let (total, of_type) = totals.pop().expect(BRANCH_TOTAL);

                                branch.new_child(tag, total, of_type)
                            }
                            Node::Sibling(tag) => branch.next_sibling(tag).expect(SIBLING_ADDED),
                            Node::Finish => branch.finish().expect(BRANCH_FINISHED),
                        }
                    }

                    match expected {
                        //
                        Expected::Matches(test) => {
                            assert_eq!(true, sut.matches(&branch), "{}", test)
                        }

                        //
                        Expected::NotMatches(test) => {
                            assert_eq!(false, sut.matches(&branch), "{}", test)
                        }

                        //
                        Expected::EndOfTests => {}
                    }
                }
            }
        }
    }

    #[inline]
    fn expected_selectors(collection: Vec<(&str, SelectorMode, Nth)>) -> Selectors {
        expected_selectors2(vec![collection])
    }

    fn expected_selectors2(collection: Vec<Vec<(&str, SelectorMode, Nth)>>) -> Selectors {
        let mut expected = HashMap::new();

        for selectors in collection {
            let mut key = Vec::new();

            let segments = selectors
                .into_iter()
                .map(|(name, mode, nth)| {
                    key.push(name.to_string());

                    make_segment(name, mode, nth).expect(VALID_SEGMENT)
                })
                .collect();

            expected
                .entry(key)
                .or_insert_with(|| Vec::new())
                .push(Segments(segments));
        }

        Selectors(expected)
    }
}
