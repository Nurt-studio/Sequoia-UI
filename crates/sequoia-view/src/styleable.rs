use sequoia_core::{Color, Edges};
use sequoia_style::BoxStyle;

pub trait Styleable: Sized {
    fn fill_width(self) -> Self;
    fn fill_height(self) -> Self;
    fn fill_size(self) -> Self {
        self.fill_width().fill_height()
    }
    fn fit(self) -> Self;
    fn hit_testable(self, on: bool) -> Self;
    fn padding(self, padding: impl Into<Edges>) -> Self;
    fn padding_xy(self, x: f32, y: f32) -> Self {
        self.padding(Edges::xy(x, y))
    }
}

pub trait Decoratable: Sized {
    fn background(self, color: Color) -> Self;
    fn border(self, width: f32, color: Color) -> Self;
    fn radius(self, radius: f32) -> Self;
    fn box_style(self, style: BoxStyle) -> Self;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traits_are_object_safe_contracts() {
        fn assert_styleable<T: Styleable>() {}
        fn assert_decoratable<T: Decoratable>() {}
        assert_styleable::<crate::views::Row>();
        assert_styleable::<crate::views::Column>();
        assert_decoratable::<crate::views::Row>();
        assert_decoratable::<crate::views::Column>();
    }
}
