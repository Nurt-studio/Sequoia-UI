use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::signal::Signal;

/// Resolved translations, keyed `locale -> key -> text`.
///
/// Nested maps let lookups borrow `&str` for both parts; the flattened
/// `(String, String)` keys allocated two `String`s on every `t()` call.
type Entries = Rc<RefCell<HashMap<String, HashMap<String, String>>>>;

#[derive(Clone)]
pub struct I18n {
    entries: Entries,
    /// Bumped by every `add`, so `t`/`has` re-run when the table changes.
    generation: Signal<u64>,
    locale: Signal<String>,
    fallback: String,
}

impl I18n {
    pub fn new(fallback: impl Into<String>) -> Self {
        let fallback = fallback.into();
        Self {
            entries: Rc::new(RefCell::new(HashMap::new())),
            generation: Signal::new(0),
            locale: Signal::new(fallback.clone()),
            fallback,
        }
    }

    pub fn add(&self, locale: &str, key: &str, value: impl Into<String>) {
        self.entries
            .borrow_mut()
            .entry(locale.to_owned())
            .or_default()
            .insert(key.to_owned(), value.into());
        // Without this bump a view built before `add` never learned about the
        // new entry and kept showing the raw key.
        self.generation.set(self.generation.get_untracked() + 1);
    }

    pub fn set_locale(&self, locale: impl Into<String>) {
        self.locale.set(locale.into());
    }

    /// Current locale, tracked like [`I18n::t`].
    pub fn locale(&self) -> String {
        self.locale.get()
    }

    pub fn locale_untracked(&self) -> String {
        self.locale.get_untracked()
    }

    pub fn locale_signal(&self) -> Signal<String> {
        self.locale.clone()
    }

    fn lookup(&self, locale: &str, key: &str) -> Option<String> {
        self.entries
            .borrow()
            .get(locale)
            .and_then(|entries| entries.get(key))
            .cloned()
    }

    pub fn t(&self, key: &str) -> String {
        let _entries = self.generation.get();
        let locale = self.locale.get();
        self.lookup(&locale, key)
            .or_else(|| self.lookup(&self.fallback, key))
            .unwrap_or_else(|| key.to_string())
    }

    pub fn t_untracked(&self, key: &str) -> String {
        let _entries = self.generation.get_untracked();
        let locale = self.locale.get_untracked();
        self.lookup(&locale, key)
            .or_else(|| self.lookup(&self.fallback, key))
            .unwrap_or_else(|| key.to_string())
    }

    pub fn has(&self, key: &str) -> bool {
        let _entries = self.generation.get();
        let locale = self.locale.get();
        self.lookup(&locale, key).is_some() || self.lookup(&self.fallback, key).is_some()
    }

    pub fn missing_in(&self, locale: &str) -> Vec<String> {
        let entries = self.entries.borrow();
        let Some(fallback) = entries.get(&self.fallback) else {
            return Vec::new();
        };
        let translated = entries.get(locale);
        let mut keys: Vec<&String> = fallback
            .keys()
            .filter(|key| translated.is_none_or(|t| !t.contains_key(*key)))
            .collect();
        keys.sort();
        keys.into_iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn falls_back_to_key_and_fallback_locale() {
        let i = I18n::new("en");
        assert_eq!(i.t_untracked("nope"), "nope");
        i.add("en", "save", "Save");
        i.set_locale("pl");
        assert_eq!(i.t_untracked("save"), "Save", "fallback en");
        i.add("pl", "save", "Zapisz");
        assert_eq!(i.t_untracked("save"), "Zapisz");
        assert!(i.has("save"));
        assert!(!i.has("nope"));
    }

    #[test]
    fn missing_lists_untranslated_keys() {
        let i = I18n::new("en");
        i.add("en", "a", "A");
        i.add("en", "b", "B");
        i.add("pl", "a", "A-pl");
        assert_eq!(i.missing_in("pl"), vec!["b".to_string()]);
        assert!(i.missing_in("en").is_empty());
    }

    #[test]
    fn adding_an_entry_wakes_readers() {
        let i = I18n::new("en");
        let runs = Rc::new(Cell::new(0u32));
        let _e = crate::effect({
            let (i, runs) = (i.clone(), runs.clone());
            move || {
                let _ = i.t("save");
                runs.set(runs.get() + 1);
            }
        });
        assert_eq!(runs.get(), 1);
        i.add("en", "save", "Save");
        assert_eq!(runs.get(), 2, "add() must notify readers of t()");
    }

    #[test]
    fn changing_the_locale_wakes_tracked_readers() {
        let i = I18n::new("en");
        i.add("en", "save", "Save");
        i.add("pl", "save", "Zapisz");
        let seen = Rc::new(std::cell::RefCell::new(String::new()));
        let _e = crate::effect({
            let (i, seen) = (i.clone(), seen.clone());
            move || *seen.borrow_mut() = i.t("save")
        });
        assert_eq!(&*seen.borrow(), "Save");
        i.set_locale("pl");
        assert_eq!(&*seen.borrow(), "Zapisz");
    }
}
