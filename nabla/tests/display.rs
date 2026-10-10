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
    #[display("{}", self.0 + 1)]
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
#[test]
fn dynamic_width_and_precision() {
    #[derive(nabla::Display)]
    #[display("{value:$>width$}|{value:+#0width$x}|{value:10.precision$}|{value:.precision$}")]
    struct Flags {
        value: u32,
        width: usize,
        precision: usize,
    }

    #[derive(nabla::Display)]
    #[display("{value:width$.precision$}|{value:🦀>width$.precision$}|{width:width$}")]
    struct Named {
        value: f64,
        width: usize,
        precision: usize,
    }

    #[derive(nabla::Display)]
    #[display("{0:width$.precision$}", width = .1, precision = .2)]
    struct Tuple(f64, usize, usize);

    #[derive(nabla::Display)]
    enum Raw {
        #[display("{value:0type$x}")]
        Value {
            value: u32,
            r#type: usize,
        },
    }

    assert_eq!(
        Named {
            value: 1.25,
            width: 6,
            precision: 1
        }
        .to_string(),
        "   1.2|🦀🦀🦀1.2|     6"
    );
    assert_eq!(Tuple(1.25, 6, 1).to_string(), "   1.2");
    assert_eq!(Raw::Value { value: 42, r#type: 4 }.to_string(), "002a");
    let value = 42;
    let width = 8;
    let precision = 3;
    assert_eq!(
        Flags {
            value,
            width,
            precision
        }
        .to_string(),
        format!("{value:$>width$}|{value:+#0width$x}|{value:10.precision$}|{value:.precision$}")
    );
}

#[test]
fn dynamic_counts_with_explicit_arguments() {
    #[derive(nabla::Display)]
    #[display("{:width$.precision$}", self.value, width = 6)]
    struct Override {
        value: f64,
        width: usize,
        precision: usize,
    }

    #[derive(nabla::Display)]
    #[display("{1:0$.2$}|{1:🦀>0$.2$}", 6, 1.25, 1)]
    struct Positional;

    #[derive(nabla::Display)]
    #[display("{:.*} {}", 1, 1.25, 42)]
    struct ImplicitPrecision;

    assert_eq!(
        Override {
            value: 1.25,
            width: 99,
            precision: 1
        }
        .to_string(),
        "   1.2"
    );
    assert_eq!(Positional.to_string(), "   1.2|🦀🦀🦀1.2");
    assert_eq!(ImplicitPrecision.to_string(), "1.2 42");
}
#[test]
fn namespaced_display_attributes() {
    #[derive(nabla::Display)]
    #[nabla(display("{value:width$}", width = 4))]
    struct Named {
        value: u32,
    }

    #[derive(nabla::Display)]
    #[nabla(display("{}", self.0 + 1))]
    struct Tuple(u32);

    #[derive(nabla::Display)]
    #[nabla(display("unit"))]
    struct Unit;

    #[derive(nabla::Display)]
    enum Enum {
        #[nabla(display("{type}"))]
        Named {
            r#type: u32,
        },
        #[nabla(display("{0}"))]
        Tuple(u32),
        #[nabla(display("unit"))]
        Unit,
        #[display("direct")]
        Direct,
    }

    assert_eq!(Named { value: 42 }.to_string(), "  42");
    assert_eq!(Tuple(41).to_string(), "42");
    assert_eq!(Unit.to_string(), "unit");
    assert_eq!(Enum::Named { r#type: 42 }.to_string(), "42");
    assert_eq!(Enum::Tuple(42).to_string(), "42");
    assert_eq!(Enum::Unit.to_string(), "unit");
    assert_eq!(Enum::Direct.to_string(), "direct");
}

#[test]
fn shadowed_core_and_write() {
    #[expect(unused_macros)]
    macro_rules! write {
        ($($tt:tt)*) => {
            compile_error!("generated code used a shadowed write! macro")
        };
    }
    mod core {}

    #[derive(nabla::Display)]
    #[display("{0}")]
    struct Wrapper(u32);

    assert_eq!(Wrapper(42).to_string(), "42");
}

mod no_prelude {
    #![no_implicit_prelude]

    extern crate alloc;
    extern crate core;
    extern crate nabla;

    #[test]
    fn display_without_prelude() {
        #[derive(nabla::Display)]
        enum Value {
            #[display("{0:>width$}", width = .1)]
            Number(u32, usize),
            #[display("{value}")]
            Named {
                value: bool,
            },
        }

        core::assert_eq!(alloc::string::ToString::to_string(&Value::Number(42, 4)), "  42");
        core::assert_eq!(
            alloc::string::ToString::to_string(&Value::Named { value: true }),
            "true"
        );
    }
}

#[test]
fn fields_in_explicit_arguments() {
    const OFFSET: u32 = 100;

    #[derive(nabla::Display)]
    enum Enum {
        #[display("{} {}", value * 2, .value + OFFSET)]
        Named {
            value: u32,
        },
        #[display("{} {} {}", _0 + 1, .1.0, .1 .1)]
        Tuple(u32, (u8, u8)),
        #[display("{:.1$}", .value, .r#type)]
        Raw {
            value: f64,
            r#type: usize,
        },
    }

    const LIMIT: u32 = 3;
    #[derive(nabla::Display)]
    #[display("{value} {} {}", LIMIT == 3, *.value == LIMIT)]
    struct Comparisons {
        value: u32,
    }

    assert_eq!(Enum::Named { value: 2 }.to_string(), "4 102");
    assert_eq!(Enum::Tuple(1, (2, 3)).to_string(), "2 2 3");
    assert_eq!(Enum::Raw { value: 1.25, r#type: 1 }.to_string(), "1.2");
    assert_eq!(Comparisons { value: 1 }.to_string(), "1 true false");
}

#[test]
fn placeholders_capture_variables() {
    const GREETING: &str = "hello";

    #[derive(nabla::Display)]
    #[display("{GREETING} {value}")]
    struct Captures {
        value: u32,
    }

    assert_eq!(Captures { value: 1 }.to_string(), "hello 1");
}
