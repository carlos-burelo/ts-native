use proc_macro::TokenStream;
use proc_macro2::TokenStream as TS2;
use quote::quote;
use syn::{parse_macro_input, parse::Parse, parse::ParseStream, FnArg, Ident, Item, ItemMod, LitStr, Token, Type};

// ── #[op("name")] ─────────────────────────────────────────────────────────────
// Used standalone (for backward compat) OR inside a #[module] mod block.
// When used standalone it still generates the old HostOp constant.
// When used inside #[module] it is consumed by the outer macro.

#[proc_macro_attribute]
pub fn op(attr: TokenStream, item: TokenStream) -> TokenStream {
    let op_name = parse_macro_input!(attr as LitStr);
    let input_fn = parse_macro_input!(item as syn::ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_attrs = &input_fn.attrs;
    let fn_block = &input_fn.block;
    let fn_output = &input_fn.sig.output;

    let param_count = input_fn.sig.inputs.iter()
        .filter(|arg| !matches!(arg, FnArg::Receiver(_)))
        .count();

    let op_const_name = format!("{}_OP", fn_name).to_uppercase();
    let op_const_ident = Ident::new(&op_const_name, fn_name.span());
    let op_alias_ident = Ident::new(&format!("{}_OP", fn_name), fn_name.span());

    let expanded = if param_count == 1 {
        let orig_name = Ident::new(&format!("original_{}", fn_name), fn_name.span());
        quote! {
            #(#fn_attrs)*
            #fn_vis fn #fn_name(_ctx: &mut dyn ::tsn_types::Context, args: &[::tsn_types::Value]) #fn_output {
                #orig_name(args)
            }
            #fn_vis fn #orig_name(args: &[::tsn_types::Value]) #fn_output #fn_block
            pub static #op_const_ident: crate::host_ops::HostOp =
                crate::host_ops::HostOp { name: #op_name, func: #fn_name };
            pub use #op_const_ident as #op_alias_ident;
        }
    } else {
        quote! {
            #input_fn
            pub static #op_const_ident: crate::host_ops::HostOp =
                crate::host_ops::HostOp { name: #op_name, func: #fn_name };
            pub use #op_const_ident as #op_alias_ident;
        }
    };

    TokenStream::from(expanded)
}

// ── #[module] ────────────────────────────────────────────────────────────────
// Attribute on a `mod` block. Generates `pub fn build() -> tsn_types::Value`.
//
// Syntax:  #[module(id = "std:math", ns = "Math")]
//          #[module(id = "std:console")]   // top-level fns, no inner namespace
//
// Inside the mod:
//   #[op("floor")]     fn floor(args: &[Value]) -> Result<Value, String> { ... }
//   #[op("floor")]     fn floor(ctx: &mut dyn Context, args: &[Value]) -> ... { ... }
//   #[export("PI")]    const PI: f64 = 3.14159...;
//   #[export("PI")]    const PI: i64 = 42;
//   #[export("TRUE")]  const T: bool = true;

struct ModuleArgs {
    ns: Option<String>,
}

impl Parse for ModuleArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut id = None;
        let mut ns = None;
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let val: LitStr = input.parse()?;
            match key.to_string().as_str() {
                "id" => id = Some(val.value()),
                "ns" => ns = Some(val.value()),
                other => return Err(syn::Error::new(key.span(), format!("unknown key '{}'", other))),
            }
            let _ = input.parse::<Token![,]>();
        }
        let _ = id.ok_or_else(|| input.error("missing `id` argument"))?;
        Ok(ModuleArgs { ns })
    }
}

#[proc_macro_attribute]
pub fn module(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ModuleArgs);
    let mod_item = parse_macro_input!(item as ItemMod);

    let (_brace, items) = match mod_item.content.clone() {
        Some(c) => c,
        None => return TokenStream::from(quote! { #mod_item }),
    };

    let mod_vis = &mod_item.vis;
    let mod_ident = &mod_item.ident;
    let ns_name_opt = &args.ns;

    // Collect ops and exports, produce clean items
    let mut ops: Vec<(String, Ident, bool)> = vec![]; // (name, fn_ident, needs_ctx)
    let mut exports: Vec<(String, TS2)> = vec![];    // (name, value_expr)
    let mut clean_items: Vec<TS2> = vec![];

    for item in &items {
        match item {
            Item::Fn(f) => {
                // Look for #[op("name")]
                let mut op_name: Option<String> = None;
                let mut clean_attrs: Vec<TS2> = vec![];
                for attr in &f.attrs {
                    if attr.path().is_ident("op") {
                        if let Ok(name) = attr.parse_args::<LitStr>() {
                            op_name = Some(name.value());
                        }
                    } else {
                        clean_attrs.push(quote! { #attr });
                    }
                }

                // Detect if fn already has ctx param (2 non-self params)
                let real_params: Vec<_> = f.sig.inputs.iter()
                    .filter(|a| !matches!(a, FnArg::Receiver(_)))
                    .collect();
                let needs_ctx = real_params.len() >= 2;

                // Emit clean fn (without #[op] attr)
                let sig = &f.sig;
                let block = &f.block;
                let vis = &f.vis;
                let fn_ident = &f.sig.ident;

                // If single-arg fn (no ctx), wrap to NativeFn signature
                if let Some(ref name) = op_name {
                    if !needs_ctx {
                        let inner = Ident::new(&format!("__inner_{}", fn_ident), fn_ident.span());
                        let out = &f.sig.output;
                        clean_items.push(quote! {
                            #(#clean_attrs)*
                            #vis fn #fn_ident(_ctx: &mut dyn ::tsn_types::Context, args: &[::tsn_types::Value]) #out {
                                #inner(args)
                            }
                            fn #inner(args: &[::tsn_types::Value]) #out #block
                        });
                    } else {
                        clean_items.push(quote! {
                            #(#clean_attrs)*
                            #vis #sig #block
                        });
                    }
                    ops.push((name.clone(), fn_ident.clone(), needs_ctx));
                } else {
                    clean_items.push(quote! { #(#clean_attrs)* #vis #sig #block });
                }
            }

            Item::Const(c) => {
                // Look for #[export("name")]
                let mut export_name: Option<String> = None;
                let mut clean_attrs: Vec<TS2> = vec![];
                for attr in &c.attrs {
                    if attr.path().is_ident("export") {
                        if let Ok(name) = attr.parse_args::<LitStr>() {
                            export_name = Some(name.value());
                        } else {
                            // #[export] with no arg → use const name
                            export_name = Some(c.ident.to_string());
                        }
                    } else {
                        clean_attrs.push(quote! { #attr });
                    }
                }

                let const_ident = &c.ident;
                let vis = &c.vis;
                let ty = &c.ty;
                let expr = &c.expr;

                if let Some(ref name) = export_name {
                    // Generate Value expression based on type
                    let val_expr = const_value_expr(ty, &quote! { #const_ident });
                    exports.push((name.clone(), val_expr));
                }

                clean_items.push(quote! {
                    #(#clean_attrs)*
                    #vis const #const_ident: #ty = #expr;
                });
            }

            other => clean_items.push(quote! { #other }),
        }
    }

    // Build function inserts for namespace
    let op_inserts: Vec<TS2> = ops.iter().map(|(name, ident, _needs_ctx)| {
        quote! {
            __obj.set_field(::std::sync::Arc::from(#name),
                ::tsn_types::Value::NativeFn(::std::boxed::Box::new(
                    (#ident as ::tsn_types::NativeFn, #name)
                )));
        }
    }).collect();

    let export_inserts: Vec<TS2> = exports.iter().map(|(name, val)| {
        quote! {
            __obj.set_field(::std::sync::Arc::from(#name), #val);
        }
    }).collect();

    // Generate build() fn
    let build_fn = if let Some(ns) = ns_name_opt {
        // Wrap everything in a namespace object
        quote! {
            pub fn build() -> ::tsn_types::Value {
                use ::std::sync::Arc;
                use ::tsn_types::value::{new_object, ObjData};
                let mut __obj = ObjData::new();
                #(#op_inserts)*
                #(#export_inserts)*
                let __ns = new_object(__obj);

                let mut __exports = ObjData::new();
                __exports.set_field(Arc::from(#ns), __ns);
                new_object(__exports)
            }
        }
    } else {
        // Top-level exports (no namespace wrapper)
        quote! {
            pub fn build() -> ::tsn_types::Value {
                use ::std::sync::Arc;
                use ::tsn_types::value::{new_object, ObjData};
                let mut __obj = ObjData::new();
                #(#op_inserts)*
                #(#export_inserts)*
                new_object(__obj)
            }
        }
    };

    TokenStream::from(quote! {
        #mod_vis mod #mod_ident {
            #(#clean_items)*
            #build_fn
        }
    })
}

/// Generate a `Value::X(...)` expression from a const's type annotation.
fn const_value_expr(ty: &Type, ident_expr: &TS2) -> TS2 {
    let ty_str = quote! { #ty }.to_string();
    match ty_str.trim() {
        "f64" | "float" =>
            quote! { ::tsn_types::Value::Float(#ident_expr as f64) },
        "i64" | "int" =>
            quote! { ::tsn_types::Value::Int(#ident_expr as i64) },
        "bool" =>
            quote! { ::tsn_types::Value::Bool(#ident_expr) },
        "& str" | "&'static str" =>
            quote! { ::tsn_types::Value::Str(::std::sync::Arc::from(#ident_expr)) },
        _ =>
            // fallback: assume f64
            quote! { ::tsn_types::Value::Float(#ident_expr as f64) },
    }
}
