//! Internationalization backend with English fallback.

use rust_i18n::Backend;

pub use rust_i18n::set_locale;
pub use rust_i18n::t;

rust_i18n::i18n!("locales", fallback = "en");

/// I18n backend with automatic English fallback.
pub struct I18nBackend;

impl Backend for I18nBackend {
  fn available_locales(&self) -> Vec<&str> {
    _RUST_I18N_BACKEND.available_locales()
  }

  fn translate(&self, locale: &str, key: &str) -> Option<&str> {
    _RUST_I18N_BACKEND
      .translate(locale, key)
      .or_else(|| _RUST_I18N_BACKEND.translate("en", key))
  }
}

/// Initialize the i18n backend.
#[macro_export]
macro_rules! init {
  () => {
    rust_i18n::i18n!(backend = codelord_i18n::I18nBackend);
  };
}

/// Get available locales.
pub fn available_locales() -> Vec<&'static str> {
  _RUST_I18N_BACKEND.available_locales()
}
