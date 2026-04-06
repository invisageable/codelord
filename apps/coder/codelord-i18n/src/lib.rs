//! IDE internationalization.

mod i18n;

pub use i18n::{I18nBackend, available_locales, set_locale, t};
