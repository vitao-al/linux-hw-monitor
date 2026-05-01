/// Thin wrapper around `gettextrs::gettext` so call-sites only need
/// `use crate::i18n::t;` and can write `t("string")`.
///
/// Returns `String`. Where GTK methods need `&str`, write `&t("…")`.
pub fn t(s: &str) -> String {
    gettextrs::gettext(s)
}
