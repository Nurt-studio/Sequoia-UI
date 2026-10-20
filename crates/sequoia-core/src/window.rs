use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct WindowId(pub u64);

impl WindowId {
    pub const PRIMARY: Self = Self(0);

    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub const fn is_primary(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "window#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_is_zero() {
        assert_eq!(WindowId::PRIMARY.get(), 0);
        assert!(WindowId::PRIMARY.is_primary());
        assert!(!WindowId::new(3).is_primary());
        assert_eq!(format!("{}", WindowId::new(7)), "window#7");
    }
}
