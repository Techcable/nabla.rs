#[derive(Copy, Clone, Debug)]
pub enum RefStyle {
    Immutable,
    #[expect(unused)]
    Mutable,
}

use darling::ast::Style;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, TokenStreamExt, quote};

impl ToTokens for RefStyle {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(self.to_token_stream());
    }
    fn to_token_stream(&self) -> TokenStream {
        match self {
            RefStyle::Immutable => quote!(ref),
            RefStyle::Mutable => quote!(ref mut),
        }
    }
}

pub trait DestructureTarget {
    type Field<'a>: FieldInfo
    where
        Self: 'a;
    fn fields(&self) -> impl Iterator<Item = Self::Field<'_>>;
    fn style(&self) -> darling::ast::Style;
}
impl<V: DestructureTarget> DestructureTarget for &V {
    type Field<'b>
        = V::Field<'b>
    where
        Self: 'b;

    fn fields(&self) -> impl Iterator<Item = Self::Field<'_>> {
        V::fields(*self)
    }

    fn style(&self) -> Style {
        V::style(*self)
    }
}
impl<F: FieldInfo + 'static> DestructureTarget for darling::ast::Fields<F> {
    type Field<'a> = &'a F;

    fn fields(&self) -> impl Iterator<Item = Self::Field<'_>> {
        self.fields.iter()
    }
    fn style(&self) -> Style {
        self.style
    }
}
impl DestructureTarget for syn::Fields {
    type Field<'a> = &'a syn::Field;

    fn fields(&self) -> impl Iterator<Item = Self::Field<'_>> {
        self.iter()
    }

    fn style(&self) -> Style {
        Style::from(self)
    }
}

pub trait FieldInfo {
    fn name(&self) -> Option<Ident>;
}
impl FieldInfo for Option<Ident> {
    fn name(&self) -> Option<Ident> {
        self.clone()
    }
}
impl FieldInfo for syn::Field {
    fn name(&self) -> Option<Ident> {
        self.ident.clone()
    }
}
impl<F: FieldInfo> FieldInfo for &'_ F {
    fn name(&self) -> Option<Ident> {
        F::name(*self)
    }
}

pub struct DestructuredType<'a, V: DestructureTarget> {
    variant: Box<V>,
    tuple_namer: Box<dyn (Fn(usize) -> String) + 'a>,
    ref_style: Option<RefStyle>,
}
impl<'a, V: DestructureTarget> DestructuredType<'a, V> {
    pub fn new(variant: V, tuple_namer: impl Fn(usize) -> String + 'a) -> Self {
        DestructuredType {
            variant: Box::new(variant),
            tuple_namer: Box::new(tuple_namer),
            ref_style: None,
        }
    }

    pub fn ref_style(&mut self, style: Option<RefStyle>) -> &mut Self {
        self.ref_style = style;
        self
    }
}
impl<V: DestructureTarget> ToTokens for DestructuredType<'_, V> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(self.to_token_stream());
    }

    fn to_token_stream(&self) -> TokenStream {
        let ref_style = self.ref_style;
        match self.variant.style() {
            Style::Tuple => {
                let fields = self.variant.fields().enumerate().map(|(index, field)| {
                    if let Some(name) = field.name() {
                        panic!("A tuple should have no named fields (got {name})")
                    } else {
                        let name = (self.tuple_namer)(index);
                        Ident::new(&name, Span::call_site())
                    }
                });
                quote! {(
                    #(#ref_style #fields,)*
                )}
            }
            Style::Struct => {
                let fields = self.variant.fields().map(|field| {
                    let name = field.name().expect("A struct should not have unnamed fields");
                    quote!(#name: #ref_style #name)
                });
                quote!({
                    #(#fields,)*
                })
            }
            Style::Unit => quote!(),
        }
    }
}
