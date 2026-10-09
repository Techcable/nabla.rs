#[test]
fn selected_newtype_variants() {
    #[derive(nabla::From, Debug, PartialEq)]
    enum Value {
        Number(#[from] u32),
        Text(#[from] String),
        OtherNumber(u32),
        Pair(u32, u64),
        Named {
            value: u32,
        },
        Unit,
    }

    assert_eq!(Value::from(42_u32), Value::Number(42));
    let text: Value = String::from("hello").into();
    assert_eq!(text, Value::Text(String::from("hello")));
    // Unmarked variants remain available, including another variant of the same source type.
    let _ = (
        Value::OtherNumber(1),
        Value::Pair(1, 2),
        Value::Named { value: 1 },
        Value::Unit,
    );
}

#[test]
fn all_newtype_variants() {
    #[derive(nabla::From, Debug, PartialEq)]
    #[from(all)]
    enum Value {
        Number(u32),
        Text(String),
        // A redundant field marker is permitted in all mode.
        Flag(#[from] bool),
    }

    assert_eq!(Value::from(42_u32), Value::Number(42));
    assert_eq!(Value::from(String::from("hello")), Value::Text(String::from("hello")));
    assert_eq!(Value::from(true), Value::Flag(true));
}

#[test]
fn raw_variant_name_and_shadowed_from() {
    struct From;

    #[derive(nabla::From)]
    enum Value {
        r#SelfValue(#[from] From),
    }

    let Value::SelfValue(From) = <Value as core::convert::From<From>>::from(From);
}

mod no_prelude {
    #![no_implicit_prelude]

    extern crate core;
    extern crate nabla;

    #[test]
    fn all_mode_without_prelude() {
        #[derive(nabla::From)]
        #[from(all)]
        enum Value {
            Number(u32),
            Flag(bool),
        }

        core::assert!(core::matches!(
            <Value as core::convert::From<u32>>::from(42),
            Value::Number(42)
        ));
        core::assert!(core::matches!(
            <Value as core::convert::From<bool>>::from(true),
            Value::Flag(true)
        ));
    }
}
#[test]
fn namespaced_from_attributes() {
    #[derive(nabla::Display, nabla::From)]
    #[nabla(display("{0}"))]
    struct Wrapper(u64);

    #[derive(nabla::From, Debug, PartialEq)]
    enum Selected {
        Number(#[nabla(from)] u32),
        Flag(#[from] bool),
        Unit,
    }

    #[derive(nabla::From, nabla::Display, Debug, PartialEq)]
    #[nabla(from(all))]
    enum All {
        #[nabla(display("{0}"))]
        Number(u32),
        #[display("{0}")]
        Flag(#[nabla(from)] bool),
    }

    assert_eq!(Selected::from(42_u32), Selected::Number(42));
    assert_eq!(Selected::from(true), Selected::Flag(true));
    let _ = Selected::Unit;
    assert_eq!(All::from(42_u32), All::Number(42));
    assert_eq!(All::from(true), All::Flag(true));
    assert_eq!(All::from(42_u32).to_string(), "42");
    assert_eq!(All::from(true).to_string(), "true");
    assert_eq!(Wrapper::from(42_u64).to_string(), "42");
}
