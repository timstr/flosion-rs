use crate::ui_core::arguments::{
    Argument, ArgumentList, FloatArgument, FloatRangeArgument, NaturalNumberArgument,
    StringIdentifierArgument,
};

fn split_vec_str(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_string).collect()
}

#[test]
fn test_empty_argument_list_empty_string() {
    let arg_list = ArgumentList::new_empty();

    let parsed_args = arg_list.parse(Vec::new());

    assert_eq!(parsed_args.values().len(), 0);
}

#[test]
fn test_empty_argument_list_nonempty_string() {
    let arg_list = ArgumentList::new_empty();

    let parsed_args = arg_list.parse(split_vec_str("foo bar baz"));

    assert_eq!(parsed_args.values().len(), 0);
}

#[test]
fn test_single_string_identifier() {
    const FOO_ARG: StringIdentifierArgument = StringIdentifierArgument("foo");

    let arg_list = ArgumentList::new_empty().add(&FOO_ARG);

    {
        let parsed_args = arg_list.parse(split_vec_str(""));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("blahblah"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, "blahblah");
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("blahblah bleep"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, "blahblah");
    }
}

#[test]
fn test_two_string_identifier() {
    const FOO_ARG: StringIdentifierArgument = StringIdentifierArgument("foo");
    const BAR_ARG: StringIdentifierArgument = StringIdentifierArgument("bar");

    let arg_list = ArgumentList::new_empty().add(&FOO_ARG).add(&BAR_ARG);

    {
        let parsed_args = arg_list.parse(split_vec_str(""));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
        assert!(parsed_args.get(&BAR_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("blahblah"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, "blahblah");

        assert!(parsed_args.get(&BAR_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("3_illegal_identifier"));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
        assert!(parsed_args.get(&BAR_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("blahblah bleep"));

        assert_eq!(parsed_args.values().len(), 2);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());
        assert_eq!(parsed_args.values()[1].0, BAR_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, "blahblah");

        let bar_val = parsed_args.get(&BAR_ARG).unwrap();

        assert_eq!(bar_val, "bleep");
    }
}

#[test]
fn test_single_float_argument() {
    const FOO_ARG: FloatArgument = FloatArgument("foo");

    let arg_list = ArgumentList::new_empty().add(&FOO_ARG);

    {
        let parsed_args = arg_list.parse(split_vec_str(""));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("not_a_float"));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("1"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, 1.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("1.0"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, 1.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("+1.0"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, 1.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("-1.0"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, -1.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("1e3"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, 1000.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("-12.34"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, -12.34);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("1 bleep"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, 1.0);
    }
}

#[test]
fn test_single_natural_number_argument() {
    const FOO_ARG: NaturalNumberArgument = NaturalNumberArgument("foo");

    let arg_list = ArgumentList::new_empty().add(&FOO_ARG);

    {
        let parsed_args = arg_list.parse(split_vec_str(""));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("not_a_natural_number"));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("1.0"));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("-1"));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("1e3"));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("0"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, 0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("1"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, 1);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("1 bleep"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, 1);
    }
}

#[test]
fn test_single_float_range_argument() {
    const FOO_ARG: FloatRangeArgument = FloatRangeArgument("foo");

    let arg_list = ArgumentList::new_empty().add(&FOO_ARG);

    {
        let parsed_args = arg_list.parse(split_vec_str(""));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("not_a_float"));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("1"));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("1-2"));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("1to2"));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("1..=2"));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("1..2"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, 1.0..=2.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("1.0..2.0"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, 1.0..=2.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("+1.0..+2.0"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, 1.0..=2.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("-2.0..-1.0"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, -2.0..=-1.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("1e2..1e3"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, 100.0..=1000.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("-12.34..56.78"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, -12.34..=56.78);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("0..1 bleep"));

        assert_eq!(parsed_args.values().len(), 1);

        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        let foo_val = parsed_args.get(&FOO_ARG).unwrap();

        assert_eq!(foo_val, 0.0..=1.0);
    }
}

#[test]
fn test_mixed_arguments() {
    const FOO_ARG: StringIdentifierArgument = StringIdentifierArgument("foo");
    const BAR_ARG: FloatArgument = FloatArgument("bar");
    const BAZ_ARG: FloatRangeArgument = FloatRangeArgument("baz");
    const QUX_ARG: NaturalNumberArgument = NaturalNumberArgument("qux");

    let arg_list = ArgumentList::new_empty()
        .add(&FOO_ARG)
        .add(&BAR_ARG)
        .add(&BAZ_ARG)
        .add(&QUX_ARG);

    {
        let parsed_args = arg_list.parse(split_vec_str(""));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&FOO_ARG).is_none());
        assert!(parsed_args.get(&BAR_ARG).is_none());
        assert!(parsed_args.get(&BAZ_ARG).is_none());
        assert!(parsed_args.get(&QUX_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("henry"));

        assert_eq!(parsed_args.values().len(), 1);
        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());

        assert_eq!(parsed_args.get(&FOO_ARG).unwrap(), "henry");
        assert!(parsed_args.get(&BAR_ARG).is_none());
        assert!(parsed_args.get(&BAZ_ARG).is_none());
        assert!(parsed_args.get(&QUX_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("henry 6.7"));

        assert_eq!(parsed_args.values().len(), 2);
        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());
        assert_eq!(parsed_args.values()[1].0, BAR_ARG.name());

        assert_eq!(parsed_args.get(&FOO_ARG).unwrap(), "henry");
        assert_eq!(parsed_args.get(&BAR_ARG).unwrap(), 6.7);
        assert!(parsed_args.get(&BAZ_ARG).is_none());
        assert!(parsed_args.get(&QUX_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("henry 6.7 0..1"));

        assert_eq!(parsed_args.values().len(), 3);
        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());
        assert_eq!(parsed_args.values()[1].0, BAR_ARG.name());
        assert_eq!(parsed_args.values()[2].0, BAZ_ARG.name());

        assert_eq!(parsed_args.get(&FOO_ARG).unwrap(), "henry");
        assert_eq!(parsed_args.get(&BAR_ARG).unwrap(), 6.7);
        assert_eq!(parsed_args.get(&BAZ_ARG).unwrap(), 0.0..=1.0);
        assert!(parsed_args.get(&QUX_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("henry 6.7 0..1 99"));

        assert_eq!(parsed_args.values().len(), 4);
        assert_eq!(parsed_args.values()[0].0, FOO_ARG.name());
        assert_eq!(parsed_args.values()[1].0, BAR_ARG.name());
        assert_eq!(parsed_args.values()[2].0, BAZ_ARG.name());
        assert_eq!(parsed_args.values()[3].0, QUX_ARG.name());

        assert_eq!(parsed_args.get(&FOO_ARG).unwrap(), "henry");
        assert_eq!(parsed_args.get(&BAR_ARG).unwrap(), 6.7);
        assert_eq!(parsed_args.get(&BAZ_ARG).unwrap(), 0.0..=1.0);
        assert_eq!(parsed_args.get(&QUX_ARG).unwrap(), 99);
    }
}

#[test]
fn test_value_name_range() {
    const NAME_ARG: StringIdentifierArgument = StringIdentifierArgument("name");
    const VALUE_ARG: FloatArgument = FloatArgument("value");
    const RANGE_ARG: FloatRangeArgument = FloatRangeArgument("range");

    let arg_list = ArgumentList::new_empty()
        .add(&NAME_ARG)
        .add(&VALUE_ARG)
        .add(&RANGE_ARG);

    {
        let parsed_args = arg_list.parse(split_vec_str(""));

        assert_eq!(parsed_args.values().len(), 0);

        assert!(parsed_args.get(&NAME_ARG).is_none());
        assert!(parsed_args.get(&VALUE_ARG).is_none());
        assert!(parsed_args.get(&RANGE_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("henry"));

        assert_eq!(parsed_args.values().len(), 1);
        assert_eq!(parsed_args.values()[0].0, NAME_ARG.name());

        assert_eq!(parsed_args.get(&NAME_ARG).unwrap(), "henry");
        assert!(parsed_args.get(&VALUE_ARG).is_none());
        assert!(parsed_args.get(&RANGE_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("6.0"));

        assert_eq!(parsed_args.values().len(), 1);
        assert_eq!(parsed_args.values()[0].0, VALUE_ARG.name());

        assert!(parsed_args.get(&NAME_ARG).is_none());
        assert_eq!(parsed_args.get(&VALUE_ARG).unwrap(), 6.0);
        assert!(parsed_args.get(&RANGE_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("5..10"));

        assert_eq!(parsed_args.values().len(), 1);
        assert_eq!(parsed_args.values()[0].0, RANGE_ARG.name());

        assert!(parsed_args.get(&NAME_ARG).is_none());
        assert!(parsed_args.get(&VALUE_ARG).is_none());
        assert_eq!(parsed_args.get(&RANGE_ARG).unwrap(), 5.0..=10.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("henry 6.0"));

        assert_eq!(parsed_args.values().len(), 2);
        assert_eq!(parsed_args.values()[0].0, NAME_ARG.name());
        assert_eq!(parsed_args.values()[1].0, VALUE_ARG.name());

        assert_eq!(parsed_args.get(&NAME_ARG).unwrap(), "henry");
        assert_eq!(parsed_args.get(&VALUE_ARG).unwrap(), 6.0);
        assert!(parsed_args.get(&RANGE_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("6.0 henry"));

        assert_eq!(parsed_args.values().len(), 2);
        assert_eq!(parsed_args.values()[0].0, VALUE_ARG.name());
        assert_eq!(parsed_args.values()[1].0, NAME_ARG.name());

        assert_eq!(parsed_args.get(&NAME_ARG).unwrap(), "henry");
        assert_eq!(parsed_args.get(&VALUE_ARG).unwrap(), 6.0);
        assert!(parsed_args.get(&RANGE_ARG).is_none());
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("henry 5..10"));

        assert_eq!(parsed_args.values().len(), 2);
        assert_eq!(parsed_args.values()[0].0, NAME_ARG.name());
        assert_eq!(parsed_args.values()[1].0, RANGE_ARG.name());

        assert_eq!(parsed_args.get(&NAME_ARG).unwrap(), "henry");
        assert!(parsed_args.get(&VALUE_ARG).is_none());
        assert_eq!(parsed_args.get(&RANGE_ARG).unwrap(), 5.0..=10.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("5..10 henry"));

        assert_eq!(parsed_args.values().len(), 2);
        assert_eq!(parsed_args.values()[0].0, RANGE_ARG.name());
        assert_eq!(parsed_args.values()[1].0, NAME_ARG.name());

        assert_eq!(parsed_args.get(&NAME_ARG).unwrap(), "henry");
        assert!(parsed_args.get(&VALUE_ARG).is_none());
        assert_eq!(parsed_args.get(&RANGE_ARG).unwrap(), 5.0..=10.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("6.0 5..10"));

        assert_eq!(parsed_args.values().len(), 2);
        assert_eq!(parsed_args.values()[0].0, VALUE_ARG.name());
        assert_eq!(parsed_args.values()[1].0, RANGE_ARG.name());

        assert!(parsed_args.get(&NAME_ARG).is_none());
        assert_eq!(parsed_args.get(&VALUE_ARG).unwrap(), 6.0);
        assert_eq!(parsed_args.get(&RANGE_ARG).unwrap(), 5.0..=10.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("5..10 6.0"));

        assert_eq!(parsed_args.values().len(), 2);
        assert_eq!(parsed_args.values()[0].0, RANGE_ARG.name());
        assert_eq!(parsed_args.values()[1].0, VALUE_ARG.name());

        assert!(parsed_args.get(&NAME_ARG).is_none());
        assert_eq!(parsed_args.get(&VALUE_ARG).unwrap(), 6.0);
        assert_eq!(parsed_args.get(&RANGE_ARG).unwrap(), 5.0..=10.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("henry 6.0 5..10"));

        assert_eq!(parsed_args.values().len(), 3);
        assert_eq!(parsed_args.values()[0].0, NAME_ARG.name());
        assert_eq!(parsed_args.values()[1].0, VALUE_ARG.name());
        assert_eq!(parsed_args.values()[2].0, RANGE_ARG.name());

        assert_eq!(parsed_args.get(&NAME_ARG).unwrap(), "henry");
        assert_eq!(parsed_args.get(&VALUE_ARG).unwrap(), 6.0);
        assert_eq!(parsed_args.get(&RANGE_ARG).unwrap(), 5.0..=10.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("henry 5..10 6.0"));

        assert_eq!(parsed_args.values().len(), 3);
        assert_eq!(parsed_args.values()[0].0, NAME_ARG.name());
        assert_eq!(parsed_args.values()[1].0, RANGE_ARG.name());
        assert_eq!(parsed_args.values()[2].0, VALUE_ARG.name());

        assert_eq!(parsed_args.get(&NAME_ARG).unwrap(), "henry");
        assert_eq!(parsed_args.get(&VALUE_ARG).unwrap(), 6.0);
        assert_eq!(parsed_args.get(&RANGE_ARG).unwrap(), 5.0..=10.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("6.0 henry 5..10"));

        assert_eq!(parsed_args.values().len(), 3);
        assert_eq!(parsed_args.values()[0].0, VALUE_ARG.name());
        assert_eq!(parsed_args.values()[1].0, NAME_ARG.name());
        assert_eq!(parsed_args.values()[2].0, RANGE_ARG.name());

        assert_eq!(parsed_args.get(&NAME_ARG).unwrap(), "henry");
        assert_eq!(parsed_args.get(&VALUE_ARG).unwrap(), 6.0);
        assert_eq!(parsed_args.get(&RANGE_ARG).unwrap(), 5.0..=10.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("6.0 5..10 henry"));

        assert_eq!(parsed_args.values().len(), 3);
        assert_eq!(parsed_args.values()[0].0, VALUE_ARG.name());
        assert_eq!(parsed_args.values()[1].0, RANGE_ARG.name());
        assert_eq!(parsed_args.values()[2].0, NAME_ARG.name());

        assert_eq!(parsed_args.get(&NAME_ARG).unwrap(), "henry");
        assert_eq!(parsed_args.get(&VALUE_ARG).unwrap(), 6.0);
        assert_eq!(parsed_args.get(&RANGE_ARG).unwrap(), 5.0..=10.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("5..10 henry 6.0"));

        assert_eq!(parsed_args.values().len(), 3);
        assert_eq!(parsed_args.values()[0].0, RANGE_ARG.name());
        assert_eq!(parsed_args.values()[1].0, NAME_ARG.name());
        assert_eq!(parsed_args.values()[2].0, VALUE_ARG.name());

        assert_eq!(parsed_args.get(&NAME_ARG).unwrap(), "henry");
        assert_eq!(parsed_args.get(&VALUE_ARG).unwrap(), 6.0);
        assert_eq!(parsed_args.get(&RANGE_ARG).unwrap(), 5.0..=10.0);
    }

    {
        let parsed_args = arg_list.parse(split_vec_str("5..10 6.0 henry"));

        assert_eq!(parsed_args.values().len(), 3);
        assert_eq!(parsed_args.values()[0].0, RANGE_ARG.name());
        assert_eq!(parsed_args.values()[1].0, VALUE_ARG.name());
        assert_eq!(parsed_args.values()[2].0, NAME_ARG.name());

        assert_eq!(parsed_args.get(&NAME_ARG).unwrap(), "henry");
        assert_eq!(parsed_args.get(&VALUE_ARG).unwrap(), 6.0);
        assert_eq!(parsed_args.get(&RANGE_ARG).unwrap(), 5.0..=10.0);
    }
}
