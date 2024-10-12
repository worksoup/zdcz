use quote::quote;
use syn::{
    punctuated::Punctuated,
    Attribute,
    Field,
    Fields,
    Meta,
    Token,
    Type,
};
/// 获取需要的属性。
/// 
/// - `iter`: 字段拥有的属性。一般为 `field.attrs.iter()`.
/// - `needed`: 所需的属性。
pub fn get_field_attr<'a>(
    iter: impl Iterator<Item = &'a Attribute>,
    needed: &str,
) -> Option<Attribute> {
    let mut b = None;
    for a in iter {
        if let Some(ident) = a.path().get_ident() {
            if ident == needed {
                b = Some(a.clone());
                break;
            } else {
                eprintln!("不支持该属性，将忽略。")
            }
        }
    }
    b
}
/// 填充字段。
/// 
/// - `fields`: 字段。
/// - `field_is_needed`: 一个函数，判断该字段是否为主要字段。
/// - `value_name`.
pub fn fill_default_fields(
    fields: &Fields,
    field_is_needed: impl Fn(&Field) -> bool,
    value_name: &proc_macro2::TokenStream,
) -> (proc_macro2::TokenStream, Vec<proc_macro2::TokenStream>) /*(fields, init)*/ {
    let mut tokens = proc_macro2::TokenStream::new();
    let mut the_instance = None;
    let mut init_expr = Vec::new();
    match fields {
        Fields::Named(fields) => {
            let fields = {
                let mut fields_ = Vec::new();
                for field in &fields.named {
                    if field_is_needed(field) && the_instance.is_none() {
                        the_instance = Some(field)
                    } else {
                        fields_.push(field);
                    }
                }
                fields_
            };
            if let Some(the_instance) = the_instance {
                let field_name = the_instance.ident.as_ref().unwrap();
                init_expr.push(quote! {let #field_name = #value_name;});
                tokens.extend(quote!(#field_name,));
            }
            for field in &fields {
                let this_attr = get_field_attr(field.attrs.iter(), "default");
                let field_name = field.ident.as_ref().unwrap();
                if let Some(this_attr) = this_attr {
                    let nested = this_attr
                        .parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
                        .expect("解析属性失败！");
                    for meta in nested {
                        match meta {
                            // #[repr(align(N))]
                            Meta::NameValue(meta) => {
                                match meta
                                    .path
                                    .get_ident()
                                    .as_ref()
                                    .expect("unrecognized default")
                                    .to_string()
                                    .as_str()
                                {
                                    "value" => {
                                        let value = meta.value;
                                        tokens.extend(quote!(#field_name: #value,))
                                    }
                                    "fn_name" => {
                                        let value = meta.value;
                                        init_expr
                                            .push(quote! {let #field_name = #value(&instance);});
                                        tokens.extend(quote!(#field_name,))
                                    }
                                    _ => {
                                        panic!("unrecognized default")
                                    }
                                }
                            }
                            _ => {
                                panic!("unrecognized default")
                            }
                        }
                    }
                } else {
                    tokens.extend(quote!(#field_name:Default::default(),))
                }
            }
            (
                quote! {
                    {#tokens}
                },
                init_expr,
            )
        }
        Fields::Unnamed(fields) => {
            fields.unnamed.iter().find_map(|f| {
                if field_is_needed(f) {
                    init_expr.push(quote! {let instance = #value_name;});
                    Some(())
                } else {
                    None
                }
            });
            let mut fields_count = 0;
            for field in &fields.unnamed {
                let this_attr = get_field_attr(field.attrs.iter(), "default");
                if field_is_needed(field) && the_instance.is_none() {
                    tokens.extend(quote!(instance,));
                    the_instance = Some(field);
                } else if let Some(this_attr) = this_attr {
                    let nested = this_attr
                        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                        .expect("解析属性失败！");
                    for meta in nested {
                        match meta {
                            // #[repr(align(N))]
                            Meta::NameValue(meta) => {
                                match meta
                                    .path
                                    .get_ident()
                                    .as_ref()
                                    .expect("unrecognized default")
                                    .to_string()
                                    .as_str()
                                {
                                    "value" => {
                                        let value = meta.value;
                                        tokens.extend(quote!(#value,))
                                    }
                                    "fn_name" => {
                                        let field_name = format!("field{fields_count}")
                                            .parse::<proc_macro2::TokenStream>()
                                            .unwrap();
                                        fields_count += 1;
                                        let value = meta.value;
                                        init_expr
                                            .push(quote! {let #field_name = #value(&instance);});
                                        tokens.extend(quote!(#field_name,))
                                    }
                                    _ => {
                                        panic!("unrecognized default")
                                    }
                                }
                            }
                            _ => {
                                panic!("unrecognized default")
                            }
                        }
                    }
                } else {
                    tokens.extend(quote!(Default::default(),));
                }
            }
            (
                quote! {
                    (#tokens)
                },
                init_expr,
            )
        }
        Fields::Unit => (quote! {}, init_expr),
    }
}
/// 判断字段类型是否为 `PhantomData` 或 `PhantomPinned`.
pub fn type_is_phantom(field: &Field) -> bool {
    if let Type::Path(ref ty) = field.ty {
        if let Some(ty) = ty.path.segments.last() {
            return ty.ident == "PhantomData" || ty.ident == "PhantomPinned";
        }
    }
    false
}
pub fn find_needed_field_index<F: Fn(&Field) -> bool>(
    fields: &Fields,
    is_need: F,
) -> (usize, usize, Option<&proc_macro2::Ident>) {
    let mut len = 0;
    let mut th = 0;
    let mut name = None;
    for (th_, field) in fields.iter().enumerate() {
        if is_need(field) {
            len += 1;
            th = th_;
            name = field.ident.as_ref();
        }
    }
    (len, th, name)
}
