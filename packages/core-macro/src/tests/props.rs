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
                "",
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
                "",
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

    #[inline]
    fn expected_selectors(collection: Vec<(&str, SelectorMode, Nth)>) -> Selectors {
        expected_selectors2(vec![collection])
    }

    fn expected_selectors2(collection: Vec<Vec<(&str, SelectorMode, Nth)>>) -> Selectors {
        let mut expected = HashMap::new();

        for selectors in collection {
            let mut key = String::new();

            let segments = selectors
                .into_iter()
                .map(|(name, mode, nth)| {
                    if key.len() > 0 {
                        key.push_str(" ");
                    }

                    key.push_str(name);

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
