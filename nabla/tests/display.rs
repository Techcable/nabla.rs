#[test]
fn raw_field_names() {
    #[derive(nabla::Display)]
    #[display("{type} {value}")]
    struct Named {
        r#type: u32,
        r#value: u32,
        r#unused: bool,
    }

    #[derive(nabla::Display)]
    enum Enum {
        #[display("{match}")]
        Named {
            r#match: u32,
        },
    }

    assert_eq!(
        Named {
            r#type: 3,
            value: 4,
            unused: false
        }
        .to_string(),
        "3 4"
    );
    assert_eq!(Enum::Named { r#match: 5 }.to_string(), "5");
}
#[test]
fn escaped_braces_around_fields() {
    #[derive(nabla::Display)]
    #[display("{{{value}}} {value}}} {{{{{value}}}}}")]
    struct Braces {
        value: u32,
    }

    assert_eq!(Braces { value: 42 }.to_string(), "{42} 42} {{42}}");
}
#[test]
fn explicit_format_arguments() {
    #[derive(nabla::Display)]
    enum Constants {
        #[display("{answer}", answer = 42)]
        Named,
        #[display("{}", 42)]
        Positional,
    }

    #[derive(nabla::Display)]
    #[display("{} {1} {0} {answer} {field}", self.field + 1, 7, answer = 6 * 7,)]
    struct Expressions {
        field: u32,
        answer: u32,
    }

    #[derive(nabla::Display)]
    #[display("{0}")]
    struct Tuple(u32);

    #[derive(nabla::Display)]
    #[display("{0}", self.0 + 1)]
    struct ExplicitTuple(u32);

    #[derive(nabla::Display)]
    #[display(
        "{field} {__nabla_arg_0} {__nabla_field_field}",
        __nabla_arg_0 = 2,
        __nabla_field_field = 3
    )]
    struct Collision {
        field: u32,
    }

    assert_eq!(Expressions { field: 4, answer: 99 }.to_string(), "5 7 5 42 4");
    assert_eq!(Tuple(3).to_string(), "3");
    assert_eq!(ExplicitTuple(3).to_string(), "4");
    assert_eq!(Collision { field: 1 }.to_string(), "1 2 3");
    assert_eq!(Constants::Named.to_string(), "42");
    assert_eq!(Constants::Positional.to_string(), "42");
}
