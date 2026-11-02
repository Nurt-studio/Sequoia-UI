/// Marker for a texture input declared in a shader signature.
///
/// A parameter named `tex` of this type is bound to group `1 + slot`, binding
/// `0` as `texture_2d<f32>` and binding `1` as a filtering sampler. In the WGSL
/// body it is available as `tex` and `tex_sampler`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Sampler2D;
