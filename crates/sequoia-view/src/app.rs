#[cfg(test)]
mod tests {
    use sequoia_reactive::Signal;

    use crate::live::SignalLiveExt;
    use crate::view::View;
    use crate::views::{Column, Row, Text};

    #[derive(Clone)]
    struct Counter {
        value: Signal<i32>,
    }

    impl View for Counter {
        fn body(&self) -> impl View + 'static {
            let label = self.value.clone();
            let inc = self.value.clone();
            Row::new()
                .child(Text::with(move || label.get().to_string()))
                .child(crate::views::Unit)
                .child(crate::live_view::LiveOpacity::new(
                    crate::views::Unit,
                    inc.map_live(|v| *v as f32),
                ))
        }
    }

    #[test]
    fn counter_component_composes() {
        let c = Counter {
            value: Signal::new(0),
        };

        let _ = c.body();
        c.value.set(3);
        assert_eq!(c.value.get_untracked(), 3);
        let _ = Column::new().child(c);
    }
}
