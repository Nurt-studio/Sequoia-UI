#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sequoia_reactive::prelude::*;

#[test]
fn signal_shared_handle() {
    let a = Signal::new(0);
    let b = a.clone();
    a.set(5);
    assert_eq!(b.get(), 5);
    assert_eq!(a.version(), 1);
}

#[test]
fn effect_writing_other_signal_runs_without_borrow_panic() {
    let a = Signal::new(0i32);
    let b = Signal::new(0i32);
    let runs = Rc::new(Cell::new(0u32));
    let _e = effect({
        let (a, b, runs) = (a.clone(), b.clone(), runs.clone());
        move || {
            runs.set(runs.get() + 1);
            let v = a.get();
            b.set(v + 1);
        }
    });
    assert_eq!(runs.get(), 1);
    assert_eq!(b.get_untracked(), 1);
    a.set(5);
    assert_eq!(runs.get(), 2);
    assert_eq!(b.get_untracked(), 6);
}

#[test]
fn signal_set_same_value_is_noop() {
    let s = Signal::new(1);
    s.set(1);
    assert_eq!(s.version(), 0);
    s.update(|v| *v += 1);
    assert_eq!(s.get(), 2);
    assert_eq!(s.version(), 1);
}

#[test]
fn get_untracked_does_not_track() {
    let a = Signal::new(1);
    let m = memo({
        let a = a.clone();
        move || a.get_untracked() * 2
    });
    assert_eq!(m.get(), 2);
    a.set(5);

    assert_eq!(m.get(), 2);
}

#[test]
fn track_subscribes_without_cloning_the_value() {
    let a = Signal::new(1);
    let runs = Rc::new(Cell::new(0u32));
    let _e = effect({
        let (a, runs) = (a.clone(), runs.clone());
        move || {
            a.track();
            runs.set(runs.get() + 1);
        }
    });
    assert_eq!(runs.get(), 1);
    a.set(2);
    assert_eq!(runs.get(), 2);
}

#[test]
fn memo_is_lazy_and_cached() {
    let a = Signal::new(2);
    let runs = Rc::new(Cell::new(0u32));
    let m = {
        let (a, runs) = (a.clone(), runs.clone());
        memo(move || {
            runs.set(runs.get() + 1);
            a.get() * 10
        })
    };
    assert_eq!(runs.get(), 0, "memo must not compute before the first get");
    assert_eq!(m.get(), 20);
    assert_eq!(runs.get(), 1);
    assert_eq!(m.get(), 20);
    assert_eq!(runs.get(), 1, "second get must hit the cache");
    a.set(3);
    assert_eq!(runs.get(), 1, "lazy memo: set does not recompute");
    assert_eq!(m.get(), 30);
    assert_eq!(runs.get(), 2);
}

#[test]
fn memo_ignores_independent_signals() {
    let a = Signal::new(1);
    let b = Signal::new(1);
    let runs = Rc::new(Cell::new(0u32));
    let m = {
        let (a, runs) = (a.clone(), runs.clone());
        memo(move || {
            runs.set(runs.get() + 1);
            a.get() * 2
        })
    };
    let _ = m.get();
    b.set(100);
    let _ = m.get();
    assert_eq!(
        runs.get(),
        1,
        "change to `b` must not touch memo depending only on `a`"
    );
}

#[test]
fn memo_equality_stops_downstream_recompute() {
    let src = Signal::new(1i32);
    let m_runs = Rc::new(Cell::new(0u32));
    let down_runs = Rc::new(Cell::new(0u32));

    let m = {
        let (src, runs) = (src.clone(), m_runs.clone());
        memo(move || {
            runs.set(runs.get() + 1);
            src.get() % 2
        })
    };
    let down = {
        let (m, runs) = (m.clone(), down_runs.clone());
        memo(move || {
            runs.set(runs.get() + 1);
            m.get() * 10
        })
    };

    assert_eq!(down.get(), 10);
    assert_eq!(m_runs.get(), 1);
    assert_eq!(down_runs.get(), 1);

    src.set(3);
    assert_eq!(down.get(), 10);
    assert_eq!(m_runs.get(), 2, "memo recomputed (dirty)");
    assert_eq!(
        down_runs.get(),
        1,
        "downstream did NOT recompute: memo version did not grow"
    );
}

#[test]
fn chained_memos_recompute_exactly_once_per_batch() {
    let a = Signal::new(1i32);
    let b_runs = Rc::new(Cell::new(0u32));
    let c_runs = Rc::new(Cell::new(0u32));

    let b = {
        let (a, runs) = (a.clone(), b_runs.clone());
        memo(move || {
            runs.set(runs.get() + 1);
            a.get() * 10
        })
    };
    let c = {
        let (b, runs) = (b.clone(), c_runs.clone());
        memo(move || {
            runs.set(runs.get() + 1);
            b.get() + 1
        })
    };

    assert_eq!(c.get(), 11);
    assert_eq!(b_runs.get(), 1);
    assert_eq!(c_runs.get(), 1);

    batch(|| a.set(2));
    assert_eq!(c.get(), 21);
    assert_eq!(b_runs.get(), 2, "b: exactly one recompute per batch");
    assert_eq!(c_runs.get(), 2, "c: exactly one recompute per batch");
    let _ = c.get();
    assert_eq!(c_runs.get(), 2);
}

#[test]
fn spurious_dirty_is_cleared_without_recompute() {
    let a = Signal::new(1i32);
    let runs = Rc::new(Cell::new(0u32));
    let m = {
        let (a, runs) = (a.clone(), runs.clone());
        memo(move || {
            runs.set(runs.get() + 1);
            a.get()
        })
    };
    let _ = m.get();
    a.set(2);
    a.set(2);
    let _ = m.get();
    assert_eq!(runs.get(), 2);

    let _ = m.get();
    assert_eq!(runs.get(), 2);
}

#[test]
fn effect_runs_on_change_and_batch_dedups() {
    let s = Signal::new(0i32);
    let runs = Rc::new(Cell::new(0u32));
    let _e = effect({
        let (s, runs) = (s.clone(), runs.clone());
        move || {
            runs.set(runs.get() + 1);
            let _ = s.get();
        }
    });
    assert_eq!(runs.get(), 1, "first run at creation");

    s.set(1);
    assert_eq!(runs.get(), 2);

    batch(|| {
        s.set(2);
        s.set(3);
        s.set(4);
    });
    assert_eq!(
        runs.get(),
        3,
        "trzy sety w batchu = jedno uruchomienie efektu"
    );
}

#[test]
fn effect_retracks_dependencies_each_run() {
    let which = Signal::new(0u8);
    let a = Signal::new(10i32);
    let b = Signal::new(20i32);
    let last = Rc::new(Cell::new(0i32));

    let _e = effect({
        let (which, a, b, last) = (which.clone(), a.clone(), b.clone(), last.clone());
        move || {
            let v = if which.get() == 0 { a.get() } else { b.get() };
            last.set(v);
        }
    });
    assert_eq!(last.get(), 10);

    a.set(11);
    assert_eq!(last.get(), 11);

    which.set(1);
    assert_eq!(last.get(), 20);

    a.set(99);
    assert_eq!(last.get(), 20, "`a` should no longer be a dependency");

    b.set(30);
    assert_eq!(last.get(), 30);
}

#[test]
fn subscription_lifecycle() {
    let s = Signal::new(0i32);
    let hits = Rc::new(Cell::new(0u32));
    let sub = s.subscribe({
        let hits = hits.clone();
        move || hits.set(hits.get() + 1)
    });
    assert_eq!(hits.get(), 1, "listener fires once at subscribe");
    s.set(1);
    assert_eq!(hits.get(), 2);
    drop(sub);
    s.set(2);
    assert_eq!(hits.get(), 2, "subscription inactive after drop");
}

#[test]
fn effects_disposed_by_drop_are_skipped_in_flush() {
    let s = Signal::new(0i32);
    let hits = Rc::new(Cell::new(0u32));
    {
        let e = effect({
            let (s, hits) = (s.clone(), hits.clone());
            move || {
                hits.set(hits.get() + 1);
                let _ = s.get();
            }
        });
        assert_eq!(hits.get(), 1);
        e.dispose();
    }
    s.set(1);
    assert_eq!(hits.get(), 1);
}

#[test]
#[should_panic(expected = "no value")]
fn memo_self_reference_panics() {
    let slot: Rc<RefCell<Option<Memo<i32>>>> = Rc::new(RefCell::new(None));
    let slot2 = slot.clone();
    let m = memo(move || {
        let inner = slot2.borrow().as_ref().unwrap().clone();
        inner.get().wrapping_add(1)
    });
    *slot.borrow_mut() = Some(m.clone());
    let _ = m.get();
}

#[test]
fn effect_writing_own_dependency_is_capped_not_panicking() {
    let s = Signal::new(0i32);
    let _e = effect({
        let s = s.clone();
        move || {
            let current = s.get();
            s.set(current + 1);
        }
    });

    assert_eq!(s.get(), 1000, "capped at MAX_EFFECT_RUNS_PER_FLUSH");
}

#[test]
fn memo_writing_own_dependency_keeps_stale_not_panicking() {
    let s = Signal::new(0i32);
    let m = {
        let s = s.clone();
        memo(move || {
            let current = s.get();
            s.set(current + 1);
            s.get()
        })
    };

    assert_eq!(m.get(), 1);
}

struct XorShift(u64);

impl XorShift {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

#[test]
fn stress_random_updates_keep_graph_consistent() {
    let mut rng = XorShift::new(0x9E37_79B9_7F4A_7C15);
    let a = Signal::new(0i64);
    let b = Signal::new(0i64);

    let m1 = {
        let (a, b) = (a.clone(), b.clone());
        memo(move || a.get() * b.get())
    };
    let m2 = {
        let (a, b) = (a.clone(), b.clone());
        memo(move || a.get() + b.get())
    };
    let m3 = {
        let (m1, m2) = (m1.clone(), m2.clone());
        memo(move || m1.get() + m2.get())
    };

    for _ in 0..20_000 {
        let r = rng.next();
        match r % 5 {
            0 => a.set(((r >> 8) % 97) as i64),
            1 => b.set(((r >> 8) % 97) as i64),
            2 => {
                let av = ((r >> 8) % 97) as i64;
                let bv = ((r >> 16) % 97) as i64;
                batch(|| {
                    a.set(av);
                    b.set(bv);
                });
            }
            3 => {
                let _ = m1.get();
            }
            _ => {
                let (av, bv) = (a.get_untracked(), b.get_untracked());
                assert_eq!(m3.get(), av * bv + av + bv, "niezmiennik grafu naruszony");
            }
        }
    }

    let (av, bv) = (a.get_untracked(), b.get_untracked());
    assert_eq!(m3.get(), av * bv + av + bv);
}

#[test]
fn transaction_batches_and_names() {
    let a = Signal::new(0);
    let runs = Rc::new(Cell::new(0u32));
    let _e = {
        let (a, runs) = (a.clone(), runs.clone());
        effect(move || {
            a.get();
            runs.set(runs.get() + 1);
        })
    };
    assert_eq!(runs.get(), 1);
    assert_eq!(current_transaction(), None);
    transaction("Move Nodes", || {
        assert_eq!(current_transaction().as_deref(), Some("Move Nodes"));
        a.set(1);
        a.set(2);
        transaction("Inner", || {
            assert_eq!(current_transaction().as_deref(), Some("Move Nodes"));
            a.set(3);
        });
    });
    assert_eq!(current_transaction(), None);
    assert_eq!(a.get(), 3);
    assert_eq!(runs.get(), 2, "one flush per transaction");
}

#[test]
fn store_mirrors_signal() {
    #[derive(Clone, PartialEq, Debug)]
    struct Doc {
        title: String,
    }
    let doc = Store::new(Doc { title: "a".into() });
    let clone = doc.clone();
    assert_eq!(doc.get().title, "a");
    doc.update(|d| d.title.push('b'));
    assert_eq!(clone.get_untracked().title, "ab");
    assert_eq!(doc.version(), 1);
    doc.set(Doc { title: "ab".into() });
    assert_eq!(doc.version(), 1, "same value = no-op");
}

#[test]
fn batch_depth_recovers_after_panic() {
    let s = Signal::new(0i32);
    let runs = Rc::new(Cell::new(0u32));
    let _e = {
        let (s, runs) = (s.clone(), runs.clone());
        effect(move || {
            s.get();
            runs.set(runs.get() + 1);
        })
    };
    assert_eq!(runs.get(), 1);

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: i32 = batch(|| panic!("boom w batchu"));
    }));
    std::panic::set_hook(hook);
    assert!(result.is_err(), "panic went through batch()");

    s.set(1);
    assert_eq!(runs.get(), 2, "flush works after panic in batch()");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        transaction("T", || panic!("boom w transakcji"));
    }));
    assert!(result.is_err());
    assert_eq!(current_transaction(), None, "transaction cleaned up");
    s.set(2);
    assert_eq!(runs.get(), 3, "flush works after transaction panic");
}

#[test]
fn flush_cutoff_unsticks_dropped_tail() {
    let sa = Signal::new(0i32);
    let sb = Signal::new(0i32);
    let counters: Vec<Rc<Cell<u32>>> = (0..1200).map(|_| Rc::new(Cell::new(0u32))).collect();
    let _effects: Vec<Effect> = counters
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let (s, c) = (if i < 1000 { sa.clone() } else { sb.clone() }, c.clone());
            effect(move || {
                s.get();
                c.set(c.get() + 1);
            })
        })
        .collect();
    assert!(
        counters.iter().all(|c| c.get() == 1),
        "each effect starts once"
    );

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    batch(|| {
        sa.set(1);
        sb.set(1);
    });
    std::panic::set_hook(hook);
    assert_eq!(counters[0].get(), 2, "the front of the queue has arrived");
    assert_eq!(counters[1199].get(), 1, "tail truncated (cycle-guard)");

    sb.set(2);
    assert!(
        counters[1000..].iter().all(|c| c.get() == 2),
        "ogon wraca po kolejnej zmianie: {:?}",
        counters[1000..].iter().map(|c| c.get()).collect::<Vec<_>>()
    );
}

#[test]
fn memo_survives_panic_and_recomputes() {
    let s = Signal::new(0i32);
    let boom = Rc::new(Cell::new(false));
    let m = {
        let (s, boom) = (s.clone(), boom.clone());
        memo(move || {
            let v = s.get();
            if boom.get() {
                panic!("boom w compute");
            }
            v * 2
        })
    };
    assert_eq!(m.get(), 0);
    boom.set(true);
    s.set(1);
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| m.get()));
    std::panic::set_hook(hook);
    assert!(r.is_err(), "panic compute goes to the calling party");

    boom.set(false);
    assert_eq!(m.get(), 2, "memo przelicza po panice, nie wisi na starej");
}

#[test]
fn effect_survives_panic_and_retries() {
    let s = Signal::new(0i32);
    let boom = Rc::new(Cell::new(false));
    let runs = Rc::new(Cell::new(0u32));
    let _e = {
        let (s, boom, runs) = (s.clone(), boom.clone(), runs.clone());
        effect(move || {
            runs.set(runs.get() + 1);
            let v = s.get();
            if boom.get() {
                panic!("boom w efekcie (v={v})");
            }
        })
    };
    assert_eq!(runs.get(), 1, "start bez paniki");

    boom.set(true);
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.set(1)));
    std::panic::set_hook(hook);
    assert!(r.is_err(), "effect panic flies to the calling");
    assert_eq!(runs.get(), 2);

    boom.set(false);
    s.set(2);
    assert_eq!(
        runs.get(),
        3,
        "the effect is alive and responds to subsequent changes"
    );
}
