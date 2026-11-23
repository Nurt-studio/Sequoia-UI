//! Compile-time typed bindings for Sequoia `.seq` applications.
//!
//! `include_seq!("Main.seq")` parses the file at compile time, embeds it (and
//! all transitive imports) with `include_str!`, and expands to a builder with
//! one typed `prop_*` method per `prop` declaration and one typed `on_*`
//! method per `on` declaration.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashSet};
use std::path::{Component, Path, PathBuf};

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use sequoia_seq::{DeferredKind, ImportDecl, SeqError};
use syn::{Ident, LitStr, parse_macro_input};

#[proc_macro]
pub fn include_seq(input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(input as LitStr);
    match expand(&path) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand(path_lit: &LitStr) -> syn::Result<TokenStream2> {
    let span = path_lit.span();
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| {
            syn::Error::new(
                span,
                "`include_seq!` needs CARGO_MANIFEST_DIR (run it from a Cargo build)",
            )
        })?;
    let main_key = normalize_display(Path::new(&path_lit.value()));
    if main_key.is_empty() {
        return Err(syn::Error::new(
            span,
            "`include_seq!` needs a `.seq` file path",
        ));
    }

    let main_abs = resolve_path(&manifest, &main_key);
    let main_src = read_source(&main_abs, span)?;
    let parsed = parse_seq_file_all_diagnostics(&main_key, &main_src, span)?;
    let app = parsed.app.as_ref().ok_or_else(|| {
        syn::Error::new(span, format!("{main_key}: missing `app Name {{ }}` block"))
    })?;

    let mut imports = BTreeMap::new();
    let mut seen = HashSet::new();
    seen.insert(main_key.clone());
    let mut stack = vec![main_key.clone()];
    collect_imports(
        &manifest,
        &main_key,
        &parsed.imports,
        &mut imports,
        &mut seen,
        &mut stack,
        span,
    )?;

    let seq = seq_crate(span)?;
    let mut methods = Vec::new();
    let mut seen_props = HashSet::new();
    let mut seen_actions = HashSet::new();
    for decl in &app.deferred {
        match decl.kind {
            DeferredKind::Prop => {
                if !seen_props.insert(decl.name.clone()) {
                    return Err(syn::Error::new(
                        span,
                        format!("{main_key}:{}: duplicate prop '{}'", decl.line, decl.name),
                    ));
                }
                methods.push(prop_method(&seq, decl, span)?);
            }
            DeferredKind::On => {
                if !seen_actions.insert(decl.name.clone()) {
                    return Err(syn::Error::new(
                        span,
                        format!(
                            "{main_key}:{}: duplicate callback '{}'",
                            decl.line, decl.name
                        ),
                    ));
                }
                methods.push(action_methods(decl, span)?);
            }
            DeferredKind::Style => {}
        }
    }

    let main_key_lit = LitStr::new(&main_key, span);
    let main_include = LitStr::new(&include_path(&manifest, &main_key), span);
    let mut import_entries = Vec::new();
    for (key, include) in &imports {
        let key_lit = LitStr::new(key, span);
        let include_lit = LitStr::new(include, span);
        import_entries.push(quote!((#key_lit, include_str!(#include_lit))));
    }

    Ok(quote! {{
        #[allow(dead_code, non_camel_case_types)]
        struct __SequoiaSeqUi {
            bindings: #seq::HostBindings,
        }

        #[allow(dead_code)]
        impl __SequoiaSeqUi {
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Self { bindings: #seq::HostBindings::new() }
            }

            /// Adds custom bindings (env, memo, rows, resources) before loading.
            pub fn configure(
                mut self,
                f: impl FnOnce(#seq::HostBindings) -> #seq::HostBindings,
            ) -> Self {
                self.bindings = f(self.bindings);
                self
            }

            #(#methods)*

            pub fn load(&self) -> ::std::result::Result<#seq::LoadedApp, #seq::SeqError> {
                #seq::SeqLoader::load_str_with_map(
                    #main_key_lit,
                    include_str!(#main_include),
                    &[#(#import_entries),*],
                    &self.bindings,
                )
            }
        }

        __SequoiaSeqUi::new()
    }})
}

fn seq_crate(span: Span) -> syn::Result<TokenStream2> {
    if let Ok(found) = crate_name("sequoia-seq") {
        return Ok(match found {
            FoundCrate::Itself => quote!(crate),
            FoundCrate::Name(name) => {
                let ident = Ident::new(&name.replace('-', "_"), span);
                quote!(::#ident)
            }
        });
    }
    if let Ok(FoundCrate::Name(name)) = crate_name("sequoia-ui") {
        let ident = Ident::new(&name.replace('-', "_"), span);
        return Ok(quote!(::#ident::seq));
    }
    Err(syn::Error::new(
        span,
        "`include_seq!` needs the `sequoia-seq` crate (or `sequoia-ui`, which re-exports it)",
    ))
}

fn prop_method(
    seq: &TokenStream2,
    decl: &sequoia_seq::DeferredDecl,
    span: Span,
) -> syn::Result<TokenStream2> {
    let (rust_ty, binding) = match decl.sig.as_slice() {
        [ty] if ty == "String" => (quote!(String), quote!(prop_str)),
        [ty] if ty == "Bool" => (quote!(bool), quote!(prop_bool)),
        [ty] if ty == "Int" => (quote!(i64), quote!(prop_int)),
        [ty] if ty == "Float" => (quote!(f64), quote!(prop_float)),
        [ty] => {
            return Err(syn::Error::new(
                span,
                format!(
                    "unknown type '{}' for prop '{}' (expected String, Bool, Int or Float)",
                    ty, decl.name
                ),
            ));
        }
        [] => {
            return Err(syn::Error::new(
                span,
                format!("prop '{}' is missing a type", decl.name),
            ));
        }
        _ => {
            return Err(syn::Error::new(
                span,
                format!("prop '{}' has more than one type", decl.name),
            ));
        }
    };
    let method = method_ident("prop_", &decl.name, span)?;
    let name = LitStr::new(&decl.name, span);
    Ok(quote! {
        pub fn #method(mut self, value: #seq::Signal<#rust_ty>) -> Self {
            self.bindings = ::std::mem::take(&mut self.bindings).#binding(#name, value);
            self
        }
    })
}

fn action_methods(decl: &sequoia_seq::DeferredDecl, span: Span) -> syn::Result<TokenStream2> {
    let signature = decl.sig.iter().map(String::as_str).collect::<Vec<_>>();
    let (binding, closure) = match signature.as_slice() {
        [] => (quote!(on), quote!(impl Fn() + 'static)),
        ["Int"] => (quote!(on_int), quote!(impl Fn(i64) + 'static)),
        ["Float"] => (quote!(on_float), quote!(impl Fn(f64) + 'static)),
        ["String"] => (quote!(on_str), quote!(impl Fn(String) + 'static)),
        ["Bool"] => (quote!(on_bool), quote!(impl Fn(bool) + 'static)),
        ["Float", "Float"] => (quote!(on_point), quote!(impl Fn(f64, f64) + 'static)),
        ["Float", "Float", "String"] => (
            quote!(on_point_named),
            quote!(impl Fn(f64, f64, String) + 'static),
        ),
        ["Float", "Float", "Float"] => (quote!(on_wheel), quote!(impl Fn(f64, f64, f64) + 'static)),
        [ty] => {
            return Err(syn::Error::new(
                span,
                format!(
                    "unknown parameter type '{}' for callback '{}' (expected Int, Float, String or Bool)",
                    ty, decl.name
                ),
            ));
        }
        _ => {
            return Err(syn::Error::new(
                span,
                format!(
                    "unsupported signature for callback '{}': use (), (Int/Float/String/Bool), (Float, Float), (Float, Float, String) or (Float, Float, Float)",
                    decl.name
                ),
            ));
        }
    };
    let method = method_ident("on_", &decl.name, span)?;
    let name = LitStr::new(&decl.name, span);
    let primary = quote! {
        pub fn #method(mut self, f: #closure) -> Self {
            self.bindings = ::std::mem::take(&mut self.bindings).#binding(#name, f);
            self
        }
    };
    if matches!(signature.as_slice(), ["Float", "Float", "Float"]) {
        let pinch_method = method_ident("on_", &format!("{}_pinch", decl.name), span)?;
        let pinch = quote! {
            pub fn #pinch_method(mut self, f: impl Fn(f64, f64, f64) + 'static) -> Self {
                self.bindings = ::std::mem::take(&mut self.bindings).on_pinch(#name, f);
                self
            }
        };
        Ok(quote! {
            #primary
            #pinch
        })
    } else {
        Ok(primary)
    }
}

fn method_ident(prefix: &str, name: &str, span: Span) -> syn::Result<Ident> {
    let raw = format!("{prefix}{name}");
    syn::parse_str::<Ident>(&raw).map_err(|_| {
        syn::Error::new(
            span,
            format!("'{name}' cannot be used to generate a Rust method name"),
        )
    })
}

fn collect_imports(
    manifest: &Path,
    parent_key: &str,
    imports: &[ImportDecl],
    out: &mut BTreeMap<String, String>,
    seen: &mut HashSet<String>,
    stack: &mut Vec<String>,
    span: Span,
) -> syn::Result<()> {
    for import in imports {
        let key = join_import(parent_key, &import.path);
        if stack.iter().any(|entry| entry == &key) {
            let mut chain = stack.clone();
            chain.push(key);
            return Err(syn::Error::new(
                span,
                format!("cyclic import: {}", chain.join(" -> ")),
            ));
        }
        if !seen.insert(key.clone()) {
            continue;
        }
        let abs = resolve_path(manifest, &key);
        let src = read_source(&abs, span)?;
        let parsed = parse_seq_file_all_diagnostics(&key, &src, span)?;
        out.insert(key.clone(), include_path(manifest, &key));
        stack.push(key.clone());
        collect_imports(manifest, &key, &parsed.imports, out, seen, stack, span)?;
        stack.pop();
    }
    Ok(())
}

fn read_source(path: &Path, span: Span) -> syn::Result<String> {
    std::fs::read_to_string(path).map_err(|e| {
        syn::Error::new(
            span,
            format!("cannot read '{}': {e}", path.to_string_lossy()),
        )
    })
}

fn parse_seq_file_all_diagnostics(
    file: &str,
    src: &str,
    span: Span,
) -> syn::Result<sequoia_seq::SeqFile> {
    let (parsed, errors) = sequoia_seq::parse_seq_file_all(file, src);
    let errors: Vec<_> = errors.into_iter().filter(SeqError::is_error).collect();
    let Some((first, rest)) = errors.split_first() else {
        return Ok(parsed);
    };
    let mut combined = seq_error(span, first);
    for error in rest {
        combined.combine(seq_error(span, error));
    }
    Err(combined)
}

fn seq_error(span: Span, error: &SeqError) -> syn::Error {
    syn::Error::new(span, error.to_string())
}

fn resolve_path(manifest: &Path, key: &str) -> PathBuf {
    let path = Path::new(key);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        manifest.join(path)
    }
}

fn include_path(manifest: &Path, key: &str) -> String {
    resolve_path(manifest, key)
        .to_string_lossy()
        .replace('\\', "/")
}

fn join_import(parent_key: &str, rel: &str) -> String {
    let requested = Path::new(rel);
    if requested.is_absolute() {
        return normalize_display(requested);
    }
    let parent = Path::new(parent_key);
    let joined = parent
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(requested);
    normalize_display(&joined)
}

fn normalize_display(path: &Path) -> String {
    let mut out = String::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if out.is_empty() || out.ends_with("..") {
                    push_display(&mut out, "..");
                } else if let Some(pos) = out.rfind('/') {
                    out.truncate(pos);
                } else {
                    out.clear();
                }
            }
            Component::RootDir => out.push('/'),
            Component::Prefix(prefix) => {
                out.push_str(&prefix.as_os_str().to_string_lossy());
            }
            Component::Normal(segment) => {
                push_display(&mut out, &segment.to_string_lossy());
            }
        }
    }
    out
}

fn push_display(out: &mut String, segment: &str) {
    if !out.is_empty() && !out.ends_with('/') {
        out.push('/');
    }
    out.push_str(segment);
}
