extern crate proc_macro;

use std::iter::FilterMap;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, punctuated::Punctuated, token::Comma, Data::Struct, DataStruct, DeriveInput,
    Field, Fields::Named, FieldsNamed, PathArguments, PathSegment,
};

#[proc_macro_derive(Builder, attributes(from_map))]
pub fn derive_builder(inp: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(inp as DeriveInput);
    let fields = match data {
        Struct(DataStruct {
            fields: Named(FieldsNamed { ref named, .. }),
            ..
        }) => named,
        _ => panic!("Not supported"),
    };

    let builders = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote! {
            pub fn #name(mut self, inp: #ty) -> Self {
                self.#name = inp;
                self
            }
        }
    });

    let builder_params = fields.iter().map(|f| {
        let name = &f.ident;
        quote! { #name }
    });
    let builder_params_build = fields.iter().map(|f| {
        let name = &f.ident;
        quote! { #name }
    });

    let res = quote! {

        impl #ident {
            #(#builders)*

            pub fn build(self) -> Self {
                let Self { #(#builder_params,)* } = self;
                Self { #(#builder_params_build,)* }
            }
        }

    };

    res.into()
}

#[proc_macro_derive(CToStr)]
pub fn derive_to_str(inp: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(inp as DeriveInput);
    let fields = match data {
        Struct(DataStruct {
            fields: Named(FieldsNamed { ref named, .. }),
            ..
        }) => named,
        _ => panic!("Not supported"),
    };

    let fields_args = fields.iter().map(|f| {
        let name = &f.ident;
        let key = format!("{}", name.clone().unwrap());
        quote! { #key, self.#name }
    });

    let buf_temp = fields
        .iter()
        .map(|_| "{},{}".to_owned())
        .collect::<Vec<String>>()
        .join(";");
    let buf = format!("[{}]", buf_temp);

    let res = quote! {
        impl ::std::fmt::Display for #ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_fmt(format_args!(
                    #buf,
                    #(#fields_args,)*
                ))
            }
        }

    };

    res.into()
}

#[proc_macro_derive(CFromStr)]
pub fn derive_from_str(inp: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(inp as DeriveInput);

    let res = quote! {
        impl FromStr for #ident {
            type Err = crate::models::system::MError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {

                let new_s = s[1..s.len() - 1].to_owned();
                let mut buf = String::new();
                let mut counter = 0;

                let data = new_s
                    .split(";")
                    .collect::<Vec<&str>>()
                    .iter()
                    .filter_map(|item: &&str| -> Option<(String, String)> {
                        let contains_open = item.chars().collect::<Vec<char>>().contains(&'[');
                        let contains_close = item.chars().collect::<Vec<char>>().contains(&']');
                        let still_open = counter > 0;

                        if contains_open && !contains_close {
                            if buf != String::new() {
                                buf.push(';');
                            }
                            buf.push_str(item);
                            counter += 1;
                        }

                        let mut strs = item.split(",").collect::<Vec<&str>>();

                        if contains_close {
                            buf.push(';');
                            buf.push_str(item);
                            counter = 0;
                            strs = match buf.split_once(',') {
                                Some(tpl) => vec![tpl.0, tpl.1],
                                None => return None,
                            };
                            let key = match strs.first() {
                                Some(k) => k.to_string(),
                                None => String::new(),
                            };

                            let val = match strs.last() {
                                Some(v) => v.to_string(),
                                None => String::new(),
                            };

                            let out: (String, String) = (key, val);

                            return Some(out);
                        }

                        if still_open {
                            return None;
                        }

                        let key = match strs.first() {
                            Some(k) => k.to_string(),
                            None => String::new(),
                        };

                        let val = match strs.last() {
                            Some(v) => v.to_string(),
                            None => String::new(),
                        };

                        let out: (String, String) = (key, val);

                        Some(out)
                    })
                    .collect::<HashMap<String, String>>();

                Ok(#ident::from_complete_map(data))
            }
        }

    };

    res.into()
}

#[proc_macro_derive(CFromMap)]
pub fn derive_from_map(inp: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(inp as DeriveInput);
    let fields = match data {
        Struct(DataStruct {
            fields: Named(FieldsNamed { ref named, .. }),
            ..
        }) => named,
        _ => panic!("Not supported"),
    };

    let _from_partial_map_args = fields.iter().map(|f| {
        let name = &f.ident;
        let key = format!("{}", name.clone().unwrap());
        quote! {
            #name: if data.contains_key(#key) { data.get(#key).unwrap() } else { Default::default() },
        }
    });

    let from_map_args = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;

        let attrs = &f.attrs;
        let mut out = quote! {};

        for attr in attrs.iter() {
            let style = attr.style;
            match style {
                syn::AttrStyle::Outer => {
                    if attr.path.is_ident("from_map") {
                        let arg = attr.path.segments.first();
                        let ident = &arg.unwrap().ident;
                        if ident.to_string() == "ignore".to_owned() {
                            out = quote! {}
                        }
                    } else {
                        let key = format!("{}", name.clone().unwrap());
                        out = quote! {
                            #name: data.get(#key).unwrap().parse::<#ty>().unwrap()
                        };
                    }
                }
                syn::AttrStyle::Inner(_) => {}
            }
        }

        if attrs.is_empty() {
            let key = format!("{}", name.clone().unwrap());
            out = quote! {
                #name: data.get(#key).unwrap().parse::<#ty>().unwrap()
            };
        }

        out
    });

    let copy_with_args = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        let key = format!("{}", name.clone().unwrap());
        quote! {
            #name: match data.get(#key) {
                Some(val) => val.parse::<#ty>().unwrap(),
                None => #name.to_owned()
            }
        }
    });

    let fields_args = fields.iter().map(|f| {
        let name = &f.ident;
        quote! {
            #name
        }
    });

    let res = quote! {
    impl crate::models::domain::FromMap for #ident {
        fn from_partial_map(data: ::std::collections::HashMap<::std::string::String, ::std::string::String>) -> Self {
            todo!()
            // Self {
            //     #(#from_partial_map_args)*
            // }
        }

        fn from_complete_map(data: ::std::collections::HashMap<::std::string::String, ::std::string::String>) -> Self {
            Self {
                #(#from_map_args,)*
            }
        }

        fn copy_with(&self, data: ::std::collections::HashMap<::std::string::String, ::std::string::String>) -> Self {
            let Self { #(#fields_args,)* } = self;
            Self {
                #(#copy_with_args,)*
            }
        }
    }
    };

    res.into()
}

#[proc_macro_derive(CToMap)]
pub fn derive_to_map(inp: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(inp as DeriveInput);
    let fields = match data {
        Struct(DataStruct {
            fields: Named(FieldsNamed { ref named, .. }),
            ..
        }) => named,
        _ => panic!("Not supported"),
    };

    let to_map_args = fields.iter().map(|f| {
        let name = &f.ident;
        let key = format!("{}", name.clone().unwrap());
        quote! {
                (#key.to_owned(), self.#name.to_string())
        }
    });

    let to_buffers_map_args = fields.iter().map(|f| {
        let name = &f.ident;
        let key = format!("{}", name.clone().unwrap());
        quote! {
                (#key.to_owned(), ::std::string::String::new())
        }
    });

    let res = quote! {
    impl crate::models::domain::ToMap for #ident {
        fn to_map(&self) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
            ::std::collections::HashMap::from([
                #(#to_map_args,)*
            ])
        }

        fn to_allowed_mutable_map(&self) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
            ::std::collections::HashMap::from([
            ])
        }

        fn to_buffers_map(&self) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
            ::std::collections::HashMap::from([
                #(#to_buffers_map_args,)*
            ])
        }
    }
    };

    res.into()
}

#[proc_macro_derive(CData, attributes(mutable_ignore))]
pub fn derive_to_table(inp: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(inp as DeriveInput);
    let fields = match data {
        Struct(DataStruct {
            fields: Named(FieldsNamed { ref named, .. }),
            ..
        }) => named,
        _ => panic!("Not supported"),
    };

    let head_attrs = fields.iter().map(|f| {
        let name = &f.ident;
        let key = format!("{}", name.clone().unwrap());

        quote! {
            #key.to_owned()
        }
    });

    let row_attrs = fields.iter().map(|f| {
        let name = &f.ident;

        quote! {
            self.#name.to_string()
        }
    });

    let head_mutable_attrs = fields.iter().filter_map(|f| {
        let name = &f.ident;
        let key = format!("{}", name.clone().unwrap());
        let attrs = &f.attrs;

        let mut out = quote! {};

        for attr in attrs.iter() {
            let style = attr.style;
            match style {
                syn::AttrStyle::Outer => {
                    let segs = attr
                        .path
                        .segments
                        .clone()
                        .into_iter()
                        .collect::<Vec<PathSegment>>();

                    for seg in segs.iter() {
                        let is_mutable = seg.ident.to_string() == "mutable_ignore";
                        if is_mutable {
                            return None;
                        }
                    }
                    out = quote! {
                        #key.to_owned()
                    };
                }
                syn::AttrStyle::Inner(_) => {}
            }
        }

        if attrs.is_empty() {
            return Some(quote! {
                #key.to_owned()
            });
        }

        return Some(out);
    });

    let res = quote! {
    impl crate::models::domain::Data for #ident {
        fn to_row(&self) -> prettytable::Row {
            prettytable::row![
                #(#row_attrs,)*
            ]
        }

        fn head(&self) -> Vec<::std::string::String> {
            vec![
                #(#head_attrs,)*
            ]
        }

        fn head_allowed_mutable(&self) -> Vec<::std::string::String> {
            vec![
                #(#head_mutable_attrs,)*
            ]
        }

        fn to_table(&self) -> prettytable::Table {
            let mut out_table = prettytable::Table::new();
            out_table.add_row(prettytable::Row::from(self.head()));
            out_table.add_row(self.to_row());
            out_table
        }
    }
    };

    res.into()
}

#[proc_macro_derive(CPartialEq, attributes(eq))]
pub fn derive_partial_eq(inp: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(inp as DeriveInput);
    let fields = match data {
        Struct(DataStruct {
            fields: Named(FieldsNamed { ref named, .. }),
            ..
        }) => named,
        _ => panic!("Not supported"),
    };

    let ne_attrs = fields.iter().filter_map(|f| {
        let name = &f.ident;
        let attrs = &f.attrs;

        for attr in attrs.iter() {
            let style = attr.style;
            match style {
                syn::AttrStyle::Outer => {
                    let segs = attr
                        .path
                        .segments
                        .clone()
                        .into_iter()
                        .collect::<Vec<PathSegment>>();

                    for seg in segs.iter() {
                        let is_mutable = seg.ident.to_string() == "eq";
                        if is_mutable {
                            return Some(quote! {
                                self.#name != other.#name
                            });
                        }
                    }
                }
                syn::AttrStyle::Inner(_) => {}
            }
        }

        if attrs.is_empty() {
            return None;
        }

        return None;
    });

    let eq_attrs = fields.iter().filter_map(|f| {
        let name = &f.ident;
        let attrs = &f.attrs;

        for attr in attrs.iter() {
            let style = attr.style;
            match style {
                syn::AttrStyle::Outer => {
                    let segs = attr
                        .path
                        .segments
                        .clone()
                        .into_iter()
                        .collect::<Vec<PathSegment>>();

                    for seg in segs.iter() {
                        let is_mutable = seg.ident.to_string() == "eq";
                        if is_mutable {
                            return Some(quote! {
                                self.#name == other.#name
                            });
                        }
                    }
                }
                syn::AttrStyle::Inner(_) => {}
            }
        }

        if attrs.is_empty() {
            return None;
        }

        return None;
    });

    let res = quote! {
    impl core::cmp::PartialEq for #ident {
        fn eq(&self, other: &Self) -> bool {
            vec![#(#eq_attrs,)*].iter().any(|cond| cond == true)
        }

        fn ne(&self, other: &Self) -> bool {
            vec![#(#ne_attrs,)*].iter().all(|cond| cond == true)
        }
    }
    };

    res.into()
}
