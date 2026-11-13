//! Function-like macro that compiles a WGSL fragment body into a typed shader.

#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use sequoia_shader_wgsl::{WGSL_EPILOGUE as EPILOGUE, WGSL_PRELUDE as PRELUDE, WGSL_STD};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Attribute, Ident, Pat, Signature, Type, Visibility, braced, parse_macro_input};

const MAX_UNIFORM_BYTES: usize = 256;
const MAX_TEXTURES: usize = 6;

struct ShaderItem {
    attrs: Vec<Attribute>,
    vis: Visibility,
    sig: Signature,
    body: TokenStream2,
}

impl Parse for ShaderItem {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let sig = input.parse()?;
        let content;
        braced!(content in input);
        let body = content.parse()?;
        Ok(Self {
            attrs,
            vis,
            sig,
            body,
        })
    }
}

#[derive(Clone, Copy)]
enum UniformType {
    F32,
    I32,
    U32,
    Bool,
    Vec2,
    Vec3,
    Vec4,
    Color,
    F32Array(usize),
    Vec4Array(usize),
}

impl UniformType {
    fn parse(ty: &Type) -> Option<Self> {
        if let Type::Path(path) = ty {
            let ident = path.path.segments.last()?.ident.to_string();
            Some(match ident.as_str() {
                "f32" => Self::F32,
                "i32" => Self::I32,
                "u32" => Self::U32,
                "bool" => Self::Bool,
                "Vec2" => Self::Vec2,
                "Vec3" => Self::Vec3,
                "Vec4" => Self::Vec4,
                "Color" => Self::Color,
                _ => return None,
            })
        } else if let Type::Array(array) = ty {
            let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(lit), .. }) = &array.len else {
                return None;
            };
            let n = lit.base10_parse::<usize>().ok()?;
            if n == 0 {
                return None;
            }
            if type_is(&array.elem, "f32") {
                Some(Self::F32Array(n))
            } else if type_is(&array.elem, "Vec4") {
                Some(Self::Vec4Array(n))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn wgsl(self) -> String {
        match self {
            Self::F32 => "f32".into(),
            Self::I32 => "i32".into(),
            Self::U32 => "u32".into(),
            Self::Bool => "u32".into(),
            Self::Vec2 => "vec2<f32>".into(),
            Self::Vec3 => "vec3<f32>".into(),
            Self::Vec4 | Self::Color => "vec4<f32>".into(),
            // WGSL requires uniform array strides to be a multiple of 16 B,
            // so f32 arrays are stored as packed vec4s and unpacked in fs_user.
            Self::F32Array(n) => format!("array<vec4<f32>, {}>", n.div_ceil(4)),
            Self::Vec4Array(n) => format!("array<vec4<f32>, {n}>"),
        }
    }

    fn align(self) -> usize {
        match self {
            Self::F32 | Self::I32 | Self::U32 | Self::Bool => 4,
            Self::Vec2 => 8,
            Self::Vec3 | Self::Vec4 | Self::Color | Self::F32Array(_) | Self::Vec4Array(_) => 16,
        }
    }

    fn size(self) -> usize {
        match self {
            Self::F32 | Self::I32 | Self::U32 | Self::Bool => 4,
            Self::Vec2 => 8,
            Self::Vec3 => 12,
            Self::Vec4 | Self::Color => 16,
            Self::F32Array(n) => n.div_ceil(4) * 16,
            Self::Vec4Array(n) => n * 16,
        }
    }
}

#[derive(Clone, Copy)]
enum TextureKind {
    Sampler2D,
    Sampler3D,
}

enum InputKind {
    FragPos,
    FragUv,
    FragWorld,
    VertexIndex,
}

enum ParamKind {
    Uniform(UniformType),
    Texture(TextureKind),
    Input(InputKind),
}

struct Param {
    name: Ident,
    kind: ParamKind,
}

struct UniformField {
    name: Ident,
    ty: UniformType,
    offset: usize,
}

struct ShaderModel {
    name: Ident,
    struct_name: Ident,
    vis: Visibility,
    fields: Vec<UniformField>,
    textures: Vec<(Ident, TextureKind)>,
    inputs: Vec<(Ident, InputKind)>,
    size: usize,
    body: TokenStream2,
    docs: Vec<Attribute>,
}

fn round_up(value: usize, align: usize) -> usize {
    value.div_ceil(align) * align
}

fn to_upper_camel(ident: &Ident) -> Ident {
    let mut out = String::new();
    for part in ident.to_string().split('_') {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.extend(chars);
        }
    }
    if out.is_empty() {
        out.push_str("Shader");
    }
    format_ident!("{out}")
}

fn type_is(ty: &Type, name: &str) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == name)
}

fn parse_params(sig: &Signature) -> syn::Result<Vec<Param>> {
    let mut params = Vec::new();
    for input in &sig.inputs {
        let syn::FnArg::Typed(typed) = input else {
            return Err(syn::Error::new(input.span(), "shader cannot take `self`"));
        };
        let Pat::Ident(pat) = typed.pat.as_ref() else {
            return Err(syn::Error::new(
                typed.pat.span(),
                "shader parameters must be named",
            ));
        };
        let name = pat.ident.clone();
        let kind = if let Some(ty) = UniformType::parse(&typed.ty) {
            ParamKind::Uniform(ty)
        } else if type_is(&typed.ty, "Sampler2D") {
            ParamKind::Texture(TextureKind::Sampler2D)
        } else if type_is(&typed.ty, "Sampler3D") {
            ParamKind::Texture(TextureKind::Sampler3D)
        } else if type_is(&typed.ty, "FragPos") || type_is(&typed.ty, "FragLocal") {
            ParamKind::Input(InputKind::FragPos)
        } else if type_is(&typed.ty, "FragUv") {
            ParamKind::Input(InputKind::FragUv)
        } else if type_is(&typed.ty, "FragWorld") {
            ParamKind::Input(InputKind::FragWorld)
        } else if type_is(&typed.ty, "VertexIndex") {
            ParamKind::Input(InputKind::VertexIndex)
        } else {
            let ty = &typed.ty;
            return Err(syn::Error::new(
                ty.span(),
                "unsupported shader parameter type; allowed: f32, i32, u32, bool, Vec2, Vec3, Vec4, Color, [f32; N], [Vec4; N], Sampler2D, Sampler3D, FragPos, FragUv, FragWorld, VertexIndex",
            ));
        };
        params.push(Param { name, kind });
    }
    Ok(params)
}

fn build_model(item: ShaderItem) -> syn::Result<ShaderModel> {
    let struct_name = to_upper_camel(&item.sig.ident);
    let params = parse_params(&item.sig)?;

    let mut fields = Vec::new();
    let mut textures = Vec::new();
    let mut inputs = Vec::new();
    let mut offset = 0usize;
    let mut seen = std::collections::HashSet::new();
    for param in params {
        let name = param.name;
        if !seen.insert(name.to_string()) {
            return Err(syn::Error::new(
                name.span(),
                format!("duplicate parameter `{name}`"),
            ));
        }
        match param.kind {
            ParamKind::Uniform(ty) => {
                offset = round_up(offset, ty.align());
                fields.push(UniformField { name, ty, offset });
                offset += ty.size();
            }
            ParamKind::Texture(kind) => {
                if textures.len() == MAX_TEXTURES {
                    return Err(syn::Error::new(
                        name.span(),
                        format!("a shader may declare at most {MAX_TEXTURES} textures"),
                    ));
                }
                textures.push((name, kind));
            }
            ParamKind::Input(kind) => inputs.push((name, kind)),
        }
    }
    let size = round_up(offset.max(16), 16);
    if size > MAX_UNIFORM_BYTES {
        return Err(syn::Error::new(
            item.sig.span(),
            format!("shader uniforms use {size} B, the limit is {MAX_UNIFORM_BYTES} B"),
        ));
    }

    let docs = item
        .attrs
        .into_iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .collect();

    Ok(ShaderModel {
        name: item.sig.ident,
        struct_name,
        vis: item.vis,
        fields,
        textures,
        inputs,
        size,
        body: item.body,
        docs,
    })
}

fn byte_write(field: &UniformField) -> TokenStream2 {
    let name = &field.name;
    let offset = field.offset;
    match field.ty {
        UniformType::F32 | UniformType::I32 | UniformType::U32 => quote! {
            bytes[#offset..#offset + 4].copy_from_slice(&self.#name.to_le_bytes());
        },
        UniformType::Bool => quote! {
            bytes[#offset..#offset + 4].copy_from_slice(&u32::from(self.#name).to_le_bytes());
        },
        UniformType::Vec2 => quote! {
            bytes[#offset..#offset + 4].copy_from_slice(&self.#name.x.to_le_bytes());
            bytes[#offset + 4..#offset + 8].copy_from_slice(&self.#name.y.to_le_bytes());
        },
        UniformType::Vec3 => quote! {
            bytes[#offset..#offset + 4].copy_from_slice(&self.#name.x.to_le_bytes());
            bytes[#offset + 4..#offset + 8].copy_from_slice(&self.#name.y.to_le_bytes());
            bytes[#offset + 8..#offset + 12].copy_from_slice(&self.#name.z.to_le_bytes());
        },
        UniformType::Vec4 => quote! {
            bytes[#offset..#offset + 4].copy_from_slice(&self.#name.x.to_le_bytes());
            bytes[#offset + 4..#offset + 8].copy_from_slice(&self.#name.y.to_le_bytes());
            bytes[#offset + 8..#offset + 12].copy_from_slice(&self.#name.z.to_le_bytes());
            bytes[#offset + 12..#offset + 16].copy_from_slice(&self.#name.w.to_le_bytes());
        },
        UniformType::Color => quote! {
            bytes[#offset..#offset + 4].copy_from_slice(&self.#name.r.to_le_bytes());
            bytes[#offset + 4..#offset + 8].copy_from_slice(&self.#name.g.to_le_bytes());
            bytes[#offset + 8..#offset + 12].copy_from_slice(&self.#name.b.to_le_bytes());
            bytes[#offset + 12..#offset + 16].copy_from_slice(&self.#name.a.to_le_bytes());
        },
        UniformType::F32Array(n) => {
            let mut tokens = TokenStream2::new();
            for i in 0..n {
                let off = offset + i * 4;
                let q = quote! {
                    bytes[#off..#off + 4].copy_from_slice(&self.#name[#i].to_le_bytes());
                };
                tokens.extend(q);
            }
            tokens
        }
        UniformType::Vec4Array(n) => {
            let mut tokens = TokenStream2::new();
            for i in 0..n {
                let off = offset + i * 16;
                let q = quote! {
                    bytes[#off..#off + 4].copy_from_slice(&self.#name[#i].x.to_le_bytes());
                    bytes[#off + 4..#off + 8].copy_from_slice(&self.#name[#i].y.to_le_bytes());
                    bytes[#off + 8..#off + 12].copy_from_slice(&self.#name[#i].z.to_le_bytes());
                    bytes[#off + 12..#off + 16].copy_from_slice(&self.#name[#i].w.to_le_bytes());
                };
                tokens.extend(q);
            }
            tokens
        }
    }
}

fn build_wgsl(model: &ShaderModel) -> (String, usize) {
    let mut wgsl = String::new();
    wgsl.push_str(PRELUDE);
    wgsl.push('\n');
    wgsl.push_str(WGSL_STD);
    wgsl.push_str("\nstruct SequoiaUniforms {\n");
    if model.fields.is_empty() {
        wgsl.push_str("    _reserved: vec4<f32>,\n");
    } else {
        for field in &model.fields {
            wgsl.push_str(&format!("    {}: {},\n", field.name, field.ty.wgsl()));
        }
    }
    wgsl.push_str("}\n");
    wgsl.push_str("@group(0) @binding(1) var<uniform> sequoia_u: SequoiaUniforms;\n");
    for (slot, (name, kind)) in model.textures.iter().enumerate() {
        let group = 1 + slot;
        let tex_type = match kind {
            TextureKind::Sampler2D => "texture_2d<f32>",
            TextureKind::Sampler3D => "texture_3d<f32>",
        };
        wgsl.push_str(&format!(
            "@group({group}) @binding(0) var {name}: {tex_type};\n"
        ));
        wgsl.push_str(&format!(
            "@group({group}) @binding(1) var {name}_sampler: sampler;\n"
        ));
    }
    wgsl.push_str("fn fs_user(frag: ShaderVarying) -> vec4<f32> {\n");
    for field in &model.fields {
        let name = &field.name;
        if matches!(field.ty, UniformType::Bool) {
            wgsl.push_str(&format!("    let {name} = sequoia_u.{name} != 0u;\n"));
        } else if let UniformType::F32Array(n) = field.ty {
            // Unpack the packed vec4 storage into a plain array so the body can
            // write `weights[i]` as if the uniform held `array<f32, N>`.
            wgsl.push_str(&format!("    var {name}: array<f32, {n}>;\n"));
            for i in 0..n {
                let lane = ["x", "y", "z", "w"][i % 4];
                wgsl.push_str(&format!(
                    "    {name}[{i}] = sequoia_u.{name}[{}].{lane};\n",
                    i / 4
                ));
            }
        } else {
            wgsl.push_str(&format!("    let {name} = sequoia_u.{name};\n"));
        }
    }
    for (name, kind) in &model.inputs {
        let value = match kind {
            InputKind::FragPos => "frag.local",
            InputKind::FragUv => "frag.uv",
            InputKind::FragWorld => "frag.world",
            InputKind::VertexIndex => {
                debug_assert!(false, "VertexIndex is not implemented; shaders receive 0");
                "0u"
            }
        };
        wgsl.push_str(&format!("    let {name} = {value};\n"));
    }
    wgsl.push('\n');
    let body_line = wgsl.lines().count() + 1;
    wgsl.push_str(&model.body.to_string());
    wgsl.push_str("\n}\n");
    wgsl.push_str(EPILOGUE);
    (wgsl, body_line)
}

fn generate(model: &ShaderModel, wgsl: &str) -> TokenStream2 {
    let runtime = runtime_path();
    let struct_name = &model.struct_name;
    let vis = &model.vis;
    let docs = &model.docs;
    let shader_name = model.name.to_string();
    let uniform_size = model.size;

    let field_defs = model.fields.iter().map(|field| {
        let name = &field.name;
        let ty = match field.ty {
            UniformType::F32 => quote!(f32),
            UniformType::I32 => quote!(i32),
            UniformType::U32 => quote!(u32),
            UniformType::Bool => quote!(bool),
            UniformType::Vec2 => quote!(#runtime::Vec2),
            UniformType::Vec3 => quote!(#runtime::Vec3),
            UniformType::Vec4 => quote!(#runtime::Vec4),
            UniformType::Color => quote!(#runtime::Color),
            UniformType::F32Array(n) => quote!([f32; #n]),
            UniformType::Vec4Array(n) => quote!([#runtime::Vec4; #n]),
        };
        quote!(pub #name: #ty,)
    });
    let struct_body = if model.fields.is_empty() {
        quote!(;)
    } else {
        quote!({ #(#field_defs)* })
    };

    let writes = model.fields.iter().map(byte_write);
    let texture_names = model.textures.iter().map(|(name, _)| name.to_string());
    let texture_kinds = model.textures.iter().map(|(_, kind)| match kind {
        TextureKind::Sampler2D => quote!(#runtime::TextureKind::D2),
        TextureKind::Sampler3D => quote!(#runtime::TextureKind::D3),
    });
    let wgsl_lit = syn::LitStr::new(wgsl, model.name.span());

    quote! {
        #(#docs)*
        #[derive(Clone, Copy, Debug)]
        #vis struct #struct_name #struct_body

        impl #runtime::SequoiShader for #struct_name {
            const NAME: &'static str = #shader_name;
            const WGSL: &'static str = #wgsl_lit;
            const TEXTURES: &'static [&'static str] = &[#(#texture_names),*];
            const TEXTURE_KINDS: &'static [#runtime::TextureKind] = &[#(#texture_kinds),*];
            const UNIFORM_SIZE: usize = #uniform_size;

            fn encode_uniforms(&self, out: &mut Vec<u8>) {
                let mut raw = [0u8; #uniform_size];
                let bytes = &mut raw;
                #(#writes)*
                out.extend_from_slice(bytes);
            }
        }
    }
}

fn runtime_path() -> TokenStream2 {
    if let Ok(found) = crate_name("sequoia-shader") {
        return match found {
            FoundCrate::Itself => quote!(crate),
            FoundCrate::Name(name) => {
                let ident = Ident::new(&name, proc_macro2::Span::call_site());
                quote!(::#ident)
            }
        };
    }
    if let Ok(FoundCrate::Name(name)) = crate_name("sequoia-ui") {
        let ident = Ident::new(&name, proc_macro2::Span::call_site());
        return quote!(::#ident::shader);
    }
    quote!(::sequoia_shader)
}

fn validate_wgsl(
    wgsl: &str,
    show: bool,
    span: proc_macro2::Span,
    body_line: usize,
) -> syn::Result<()> {
    let body_line = body_line as u32;
    let user_line = |line: u32| {
        line.checked_sub(body_line)
            .map_or(line, |offset| offset + 1)
    };
    let module = match naga::front::wgsl::parse_str(wgsl) {
        Ok(module) => module,
        Err(error) => {
            let detail = match error.location(wgsl) {
                Some(location) => format!(
                    "{} (line {}, column {})",
                    error,
                    user_line(location.line_number),
                    location.line_position
                ),
                None => error.to_string(),
            };
            let mut message = format!("WGSL error: {detail}");
            if show {
                message.push_str("\n\nGenerated WGSL:\n");
                message.push_str(wgsl);
            }
            return Err(syn::Error::new(span, message));
        }
    };
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    if let Err(error) = validator.validate(&module) {
        let detail = match error.location(wgsl) {
            Some(location) => format!(
                "{} (line {}, column {})",
                error, location.line_number, location.line_position
            ),
            None => error.to_string(),
        };
        let mut message = format!("WGSL validation error: {detail}");
        if show {
            message.push_str("\n\nGenerated WGSL:\n");
            message.push_str(wgsl);
        }
        return Err(syn::Error::new(span, message));
    }
    let has_fragment = module
        .entry_points
        .iter()
        .any(|entry| entry.name == "fs_main" && entry.stage == naga::ShaderStage::Fragment);
    if !has_fragment {
        return Err(syn::Error::new(
            span,
            "WGSL error: missing `fs_main` entry point",
        ));
    }
    Ok(())
}

/// Turns a function with a raw WGSL body into a typed `SequoiShader` struct.
///
/// Parameters become either uniforms, texture inputs, or fragment built-ins.
/// The body is inserted verbatim into `fn fs_user`; no Rust-to-WGSL translation
/// is performed.
///
/// This is a function-like macro so the WGSL body does not have to parse as
/// Rust. Invoke it as `shader! { fn name(...) -> Color { ... } }`.
#[proc_macro]
pub fn shader(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ShaderItem);
    let span = item.sig.span();
    let model = match build_model(item) {
        Ok(model) => model,
        Err(error) => return error.to_compile_error().into(),
    };
    let (wgsl, body_line) = build_wgsl(&model);
    if let Err(error) = validate_wgsl(&wgsl, false, span, body_line) {
        return error.to_compile_error().into();
    }
    generate(&model, &wgsl).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compile(source: &str) -> syn::Result<String> {
        let item: ShaderItem = syn::parse_str(source)?;
        let model = build_model(item)?;
        let (wgsl, body_line) = build_wgsl(&model);
        validate_wgsl(&wgsl, false, model.name.span(), body_line)?;
        Ok(wgsl)
    }

    #[test]
    fn invalid_body_reports_line_and_column() {
        let error = compile("fn broken() -> Color { return nope(frag.uv); }")
            .expect_err("invalid WGSL must fail");
        let message = error.to_string();
        assert!(message.contains("WGSL"), "{message}");
        assert!(message.contains("line 1"), "{message}");
        assert!(message.contains("column"), "{message}");
    }

    #[test]
    fn unsupported_parameter_is_reported() {
        let error = compile("fn broken(value: Foo) -> Color { return vec4<f32>(1.0); }")
            .expect_err("unsupported parameter must fail");
        assert!(error.to_string().contains("unsupported"), "{error}");
    }

    #[test]
    fn unknown_function_is_rejected() {
        let error = compile("fn broken(uv: FragUv) -> Color { return mystery(uv); }")
            .expect_err("unknown WGSL function must fail");
        let message = error.to_string();
        assert!(message.contains("mystery"), "{message}");
        assert!(message.contains("line"), "{message}");
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "VertexIndex is not implemented")]
    fn vertex_index_fallback_is_not_silent() {
        let _ = compile("fn broken(index: VertexIndex) -> Color { return vec4<f32>(1.0); }");
    }

    #[test]
    fn f32_arrays_are_packed_into_vec4_storage() {
        let wgsl = compile(
            "fn arr(weights: [f32; 5]) -> Color { return vec4<f32>(weights[4]); }",
        )
        .expect("arrays must compile");
        assert!(wgsl.contains("weights: array<vec4<f32>, 2>,"), "{wgsl}");
        assert!(wgsl.contains("var weights: array<f32, 5>;"), "{wgsl}");
        assert!(wgsl.contains("weights[4] = sequoia_u.weights[1].x;"), "{wgsl}");
    }

    #[test]
    fn missing_return_is_rejected() {
        let error = compile("fn broken() -> Color { let x = vec4<f32>(1.0); }")
            .expect_err("missing return must fail");
        assert!(error.to_string().contains("WGSL"), "{error}");
    }

    #[test]
    fn mismatched_operand_types_are_rejected() {
        let error = compile(
            "fn broken(color: Color) -> Color { return vec4<f32>(color.rgb + vec2<f32>(1.0, 2.0), 1.0); }",
        )
        .expect_err("vec3 plus vec2 must fail");
        let message = error.to_string();
        assert!(message.contains("line"), "{message}");
        assert!(message.contains("column"), "{message}");
    }
}
