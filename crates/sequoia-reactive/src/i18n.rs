use std::collections::HashMap;

use crate::signal::Signal;

#[derive(Clone)]
pub struct I18n {
    entries: std::rc::Rc<std::cell::RefCell<HashMap<(String, String), String>>>,
    locale: Signal<String>,
    fallback: String,
}

impl I18n {
    pub fn new(fallback: impl Into<String>) -> Self {
        let fallback = fallback.into();
        Self {
            entries: Default::default(),
            locale: Signal::new(fallback.clone()),
            fallback,
        }
    }

    pub fn add(&self, locale: &str, key: &str, value: impl Into<String>) {
        self.entries
            .borrow_mut()
            .insert((locale.to_string(), key.to_string()), value.into());
    }

    pub fn set_locale(&self, locale: impl Into<String>) {
        self.locale.set(locale.into());
    }

    pub fn locale(&self) -> String {
        self.locale.get_untracked()
    }

    pub fn locale_signal(&self) -> Signal<String> {
        self.locale.clone()
    }

    fn lookup(&self, locale: &str, key: &str) -> Option<String> {
        self.entries
            .borrow()
            .get(&(locale.to_string(), key.to_string()))
            .cloned()
    }

    pub fn t(&self, key: &str) -> String {
        let locale = self.locale.get();
        self.lookup(&locale, key)
            .or_else(|| self.lookup(&self.fallback, key))
            .unwrap_or_else(|| key.to_string())
    }

    pub fn t_untracked(&self, key: &str) -> String {
        let locale = self.locale.get_untracked();
        self.lookup(&locale, key)
            .or_else(|| self.lookup(&self.fallback, key))
            .unwrap_or_else(|| key.to_string())
    }

    pub fn has(&self, key: &str) -> bool {
        let locale = self.locale.get_untracked();
        self.lookup(&locale, key).is_some() || self.lookup(&self.fallback, key).is_some()
    }

    pub fn missing_in(&self, locale: &str) -> Vec<String> {
        let entries = self.entries.borrow();
        let mut fallback_keys: Vec<String> = entries
            .keys()
            .filter(|(l, _)| l == &self.fallback)
            .map(|(_, k)| k.clone())
            .collect();
        fallback_keys.sort();
        fallback_keys
            .into_iter()
            .filter(|k| !entries.contains_key(&(locale.to_string(), k.clone())))
            .collect()
    }
}

#[cfg(test)]
mod tests {
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
}
