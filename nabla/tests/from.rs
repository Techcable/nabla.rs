#![no_std]
#![no_implicit_prelude]

extern crate core;
extern crate nabla;

#[test]
fn without_prelude() {
    #[derive(nabla::From)]
    struct Wrapper(u32);

    core::assert_eq!(<Wrapper as core::convert::From<u32>>::from(42).0, 42);
}

#[test]
fn with_shadowed_from() {
    struct From;

    #[derive(nabla::From)]
    struct Wrapper(From);

    let Wrapper(From) = <Wrapper as core::convert::From<From>>::from(From);
}
