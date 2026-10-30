use sequoia_reactive::{Memo, Signal};

use crate::live::{AnimSpec, Live, SpringSpec};

pub type Value<T> = Live<T>;

pub fn value<T: Clone + PartialEq + 'static>(v: T) -> Value<T> {
    Live::fixed(v)
}

pub fn derived<T: Clone + PartialEq + 'static>(f: impl Fn() -> T + 'static) -> Value<T> {
    Live::derived(f)
}

pub trait ValueExt<T: Clone + PartialEq + 'static>: Clone {
    fn value_untracked(&self) -> T;

    /// Derives a new value from this one.
    ///
    /// The closure receives `&T`, not `T`, so match it against a reference
    /// (for example `|v| *v * 2.0` or `|v| v.is_open()`).
    fn map_value<U: Clone + PartialEq + 'static>(&self, f: impl Fn(&T) -> U + 'static) -> Value<U>;

    fn animated(self, spec: impl Into<AnimSpec>) -> Value<T>;

    fn springed(self, spec: SpringSpec) -> Value<T>;
}

impl<T: Clone + PartialEq + 'static> ValueExt<T> for Signal<T> {
    fn value_untracked(&self) -> T {
        self.get_untracked()
    }
    fn map_value<U: Clone + PartialEq + 'static>(&self, f: impl Fn(&T) -> U + 'static) -> Value<U> {
        let s = self.clone();
        Live::derived(move || f(&s.get()))
    }
    fn animated(self, spec: impl Into<AnimSpec>) -> Value<T> {
        Live::of_signal(self).animate(spec)
    }
    fn springed(self, spec: SpringSpec) -> Value<T> {
        Live::of_signal(self).spring(spec)
    }
}

impl<T: Clone + PartialEq + 'static> ValueExt<T> for Memo<T> {
    fn value_untracked(&self) -> T {
        self.get_untracked()
    }
    fn map_value<U: Clone + PartialEq + 'static>(&self, f: impl Fn(&T) -> U + 'static) -> Value<U> {
        let m = self.clone();
        Live::derived(move || f(&m.get()))
    }
    fn animated(self, spec: impl Into<AnimSpec>) -> Value<T> {
        Live::of_memo(self).animate(spec)
    }
    fn springed(self, spec: SpringSpec) -> Value<T> {
        Live::of_memo(self).spring(spec)
    }
}

pub fn spring_spec(s: sequoia_animation::Spring) -> SpringSpec {
    let w0 = std::f32::consts::TAU * s.frequency.max(0.0001);
    let k = w0 * w0;
    let c = 2.0 * s.damping_ratio.max(0.0) * w0;
    SpringSpec {
        stiffness: k,
        damping: c,
        mass: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_accepts_static_reactive_and_animated() {
        fn takes_f32(_: impl Into<Value<f32>>) {}
        let s = Signal::new(0.5f32);
        takes_f32(0.8);
        takes_f32(s.clone());
        takes_f32(s.clone().animated(AnimSpec::SNAPPY));
        takes_f32(s.clone().springed(SpringSpec::BOUNCY));
        takes_f32(s.map_value(|v| v * 2.0));
        takes_f32(value(0.1));
        takes_f32(derived(|| 0.2));
    }

    #[test]
    fn presets_are_sane() {
        const { assert!(AnimSpec::FAST.duration_secs <= AnimSpec::SMOOTH.duration_secs) };
        const { assert!(AnimSpec::SMOOTH.duration_secs <= AnimSpec::SLOW.duration_secs) };
        const { assert!(AnimSpec::SNAPPY.duration_secs <= AnimSpec::GENTLE.duration_secs) };
        let _ = (
            AnimSpec::fast(),
            AnimSpec::smooth(),
            AnimSpec::snappy(),
            AnimSpec::gentle(),
        );
        let _ = (
            SpringSpec::snappy(),
            SpringSpec::bouncy(),
            SpringSpec::gentle(),
        );
    }

    #[test]
    fn spring_converts_without_nan() {
        for s in [
            sequoia_animation::Spring::snappy(),
            sequoia_animation::Spring::bouncy(),
            sequoia_animation::Spring::gentle(),
        ] {
            let spec = spring_spec(s);
            assert!(spec.stiffness.is_finite() && spec.stiffness > 0.0);
            assert!(spec.damping.is_finite() && spec.damping >= 0.0);
        }
    }

    #[test]
    fn value_ext_maps_and_animates() {
        let s = Signal::new(false);
        let v = s.map_value(|x| if *x { 1.0f32 } else { 0.6 });
        assert_eq!(v.target_untracked(), 0.6);
        s.set(true);
        assert_eq!(v.target_untracked(), 1.0);
        assert!(s.value_untracked());
        let a = s.clone().animated(AnimSpec::SMOOTH);
        assert!(a.is_animated());
    }
}
