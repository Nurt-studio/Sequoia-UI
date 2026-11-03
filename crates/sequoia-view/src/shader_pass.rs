//! Typed shader passes and chains for [`Canvas`](crate::Canvas).
//!
//! A [`ShaderPass`] bundles a generated shader with its rectangle, blend mode,
//! UV window, and texture bindings. [`ShaderChain`] sequences several passes,
//! feeding each result into the next one as an offscreen target.

use std::rc::Rc;

use sequoia_core::{BlendMode, Rect, TexInput};
use sequoia_shader::SequoiShader;

use crate::painter::Painter;

/// A configured shader draw that can be handed to a canvas or reused.
///
/// Inputs are optional: [`Painter`] samples transparent white for textures the
/// pass does not bind. [`Self::primary_input`] binds the shader's first
/// declared texture, which is what most single-input effects need.
#[derive(Clone)]
pub struct ShaderPass<S: SequoiShader> {
    shader: S,
    rect: Option<Rect>,
    uv: Option<[f32; 4]>,
    blend: Option<BlendMode>,
    inputs: Vec<(&'static str, TexInput)>,
}

impl<S: SequoiShader> ShaderPass<S> {
    pub fn new(shader: S) -> Self {
        Self {
            shader,
            rect: None,
            uv: None,
            blend: None,
            inputs: Vec::new(),
        }
    }

    /// Draws into `rect`. Without it the canvas bounds are used.
    pub fn at(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    /// Samples the shader inputs with an explicit UV window.
    pub fn uv(mut self, uv: [f32; 4]) -> Self {
        self.uv = Some(uv);
        self
    }

    /// Overrides the blend mode for this pass only.
    pub fn blend(mut self, blend: BlendMode) -> Self {
        self.blend = Some(blend);
        self
    }

    /// Binds `input` to the shader texture parameter `name`.
    pub fn input(mut self, name: &'static str, input: TexInput) -> Self {
        if let Some(existing) = self.inputs.iter_mut().find(|(key, _)| *key == name) {
            existing.1 = input;
        } else {
            self.inputs.push((name, input));
        }
        self
    }

    /// Binds the first texture declared by the shader, unless already bound.
    ///
    /// This removes the need to repeat the parameter name at every call site.
    pub fn primary_input(mut self, input: TexInput) -> Self {
        if let Some(name) = S::TEXTURES.first().copied() {
            if !self.inputs.iter().any(|(key, _)| *key == name) {
                self.inputs.push((name, input));
            }
        }
        self
    }

    /// Binds the first two declared textures in order.
    pub fn layers(self, first: TexInput, second: TexInput) -> Self {
        let mut pass = self;
        if let Some(name) = S::TEXTURES.first().copied() {
            pass = pass.input(name, first);
        }
        if let Some(name) = S::TEXTURES.get(1).copied() {
            pass = pass.input(name, second);
        }
        pass
    }

    pub fn name(&self) -> &'static str {
        S::NAME
    }

    pub fn target_rect(&self) -> Option<Rect> {
        self.rect
    }

    /// Records the draw. Returns `false` when the shader is not registered on
    /// the runtime or the rectangle is empty.
    pub fn draw(&self, g: &mut Painter) -> bool
    where
        S: Copy,
    {
        let rect = self.rect.unwrap_or_else(|| g.rect());
        let inputs: Vec<(&str, TexInput)> = self
            .inputs
            .iter()
            .map(|(name, input)| (*name, input.clone()))
            .collect();
        match (self.uv, self.blend) {
            (Some(uv), Some(blend)) => {
                g.try_shader_at_with_blend(self.shader, rect, uv, &inputs, blend)
            }
            (Some(uv), None) => g.try_shader_at(self.shader, rect, uv, &inputs),
            (None, Some(blend)) => g.try_shader_with_blend(self.shader, rect, &inputs, blend),
            (None, None) => g.try_shader(self.shader, rect, &inputs),
        }
    }
}

type ChainPass = Rc<dyn Fn(&mut Painter, Rect, TexInput) -> bool>;

/// An ordered list of shader effects applied to captured content.
///
/// Each pass receives the previous pass output as a texture target. A pass that
/// is not registered is skipped and its input is forwarded to the next pass. If
/// the final pass is missing, the captured layer is blitted back unchanged, so
/// a missing optional effect degrades instead of blanking the layer.
#[derive(Clone, Default)]
pub struct ShaderChain {
    passes: Vec<ChainPass>,
}

impl ShaderChain {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an effect that binds the shader's first declared texture.
    pub fn effect<S: SequoiShader + Copy + 'static>(self, shader: S) -> Self {
        self.push(shader, None, None)
    }

    /// Appends an effect with an explicit blend mode.
    pub fn effect_with_blend<S: SequoiShader + Copy + 'static>(
        self,
        shader: S,
        blend: BlendMode,
    ) -> Self {
        self.push(shader, None, Some(blend))
    }

    /// Appends an effect that binds `input` to a named texture parameter.
    pub fn effect_to<S: SequoiShader + Copy + 'static>(
        self,
        shader: S,
        input: &'static str,
        blend: Option<BlendMode>,
    ) -> Self {
        self.push(shader, Some(input), blend)
    }

    fn push<S: SequoiShader + Copy + 'static>(
        mut self,
        shader: S,
        input: Option<&'static str>,
        blend: Option<BlendMode>,
    ) -> Self {
        let input = input.unwrap_or_else(|| S::TEXTURES.first().copied().unwrap_or("content"));
        let pass = Rc::new(
            move |g: &mut Painter, rect: Rect, target: TexInput| -> bool {
                let inputs = [(input, target)];
                match blend {
                    Some(blend) => g.try_shader_with_blend(shader, rect, &inputs, blend),
                    None => g.try_shader(shader, rect, &inputs),
                }
            },
        );
        self.passes.push(pass);
        self
    }

    /// Appends every pass from `other` after the current ones.
    pub fn extend(&mut self, other: ShaderChain) {
        self.passes.extend(other.passes);
    }

    pub fn len(&self) -> usize {
        self.passes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.passes.is_empty()
    }

    pub(crate) fn passes(&self) -> &[ChainPass] {
        &self.passes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_core::RenderTargetId;
    use sequoia_shader::shader;

    shader! {
        fn invert(uv: FragUv, content: Sampler2D) -> Color {
            let c = textureSample(content, content_sampler, uv);
            return vec4<f32>(1.0 - c.rgb, c.a);
        }
    }

    #[test]
    fn primary_input_does_not_override_explicit_binding() {
        let pass = ShaderPass::new(Invert)
            .input("content", TexInput::Target(RenderTargetId::new(7)))
            .primary_input(TexInput::Target(RenderTargetId::new(9)));
        assert_eq!(pass.inputs.len(), 1);
        assert_eq!(pass.inputs[0].1, TexInput::Target(RenderTargetId::new(7)));
    }

    #[test]
    fn primary_input_uses_the_declared_parameter_name() {
        let pass = ShaderPass::new(Invert).primary_input(TexInput::Target(RenderTargetId::new(3)));
        assert_eq!(
            pass.inputs,
            vec![("content", TexInput::Target(RenderTargetId::new(3)))]
        );
        assert_eq!(pass.name(), Invert::NAME);
    }

    #[test]
    fn chain_keeps_order_and_can_be_extended() {
        let mut chain = ShaderChain::new()
            .effect(Invert)
            .effect_with_blend(Invert, BlendMode::Add);
        assert_eq!(chain.len(), 2);
        chain.extend(ShaderChain::new().effect(Invert));
        assert_eq!(chain.len(), 3);
        assert!(!chain.is_empty());
    }
}
