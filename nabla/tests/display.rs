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
