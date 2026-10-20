use std::time::Duration;

pub trait DurationExt {
    fn ms(self) -> Duration;

    fn secs(self) -> Duration;
}

impl DurationExt for u64 {
    fn ms(self) -> Duration {
        Duration::from_millis(self)
    }

    fn secs(self) -> Duration {
        Duration::from_secs(self)
    }
}

impl DurationExt for f64 {
    fn ms(self) -> Duration {
        Duration::from_secs_f64(self / 1000.0)
    }

    fn secs(self) -> Duration {
        Duration::from_secs_f64(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions() {
        assert_eq!(250.ms(), Duration::from_millis(250));
        assert_eq!(2.secs(), Duration::from_secs(2));

        assert_eq!(1500.0.ms(), Duration::from_millis(1500));
        assert_eq!(1.5.secs(), Duration::from_millis(1500));
    }
}
