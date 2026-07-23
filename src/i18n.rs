use gettextrs::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translation_works() {
        // Only test if .mo file exists on this system
        let mo = std::path::Path::new("/usr/share/locale/fa/LC_MESSAGES/mirrorman.mo");
        if !mo.exists() {
            eprintln!("SKIP: .mo file not found");
            return;
        }

        let old_lang = std::env::var("LANG").ok();
        let old_language = std::env::var("LANGUAGE").ok();
        std::env::set_var("LANG", "fa_IR");
        std::env::remove_var("LANGUAGE");

        init();

        let translated = gettext("About");
        eprintln!("DEBUG: gettext('About') = '{translated}'");
        assert_ne!(translated, "About", "Translation failed - string not translated");

        if let Some(v) = old_lang { std::env::set_var("LANG", v); } else { std::env::remove_var("LANG"); }
        if let Some(v) = old_language { std::env::set_var("LANGUAGE", v); }
    }
}

static CACHE: once_cell::sync::Lazy<RwLock<HashMap<&'static str, &'static str>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

pub fn init() {
    // LANGUAGE overrides LANG in gettext's lookup order; unset it so LANG
    // is the sole determinant of the locale.
    std::env::remove_var("LANGUAGE");
    setlocale(LocaleCategory::LcAll, "");
    let locale_dir = detect_locale_dir();
    bindtextdomain("mirrorman", &locale_dir).expect("Failed to bind text domain");
    textdomain("mirrorman").expect("Failed to set text domain");
}

fn detect_locale_dir() -> String {
    // Check several likely locations for the .mo file
    let mut candidates: Vec<PathBuf> = vec![
        PathBuf::from("/usr/share/locale"), // system-wide install
    ];

    // Relative to the executable (handles target/debug/, target/release/, /usr/bin/)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            // /usr/bin/ → /usr/share/locale (already covered above)
            // target/debug/ → walk up to project root
            for ancestor in parent.ancestors().skip(1).take(3) {
                let candidate = ancestor.join("locale");
                if !candidates.contains(&candidate) {
                    candidates.push(candidate);
                }
            }
        }
    }

    // Current working directory (running from project root)
    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join("locale");
        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }

    for dir in &candidates {
        if dir.join("fa").join("LC_MESSAGES").join("mirrorman.mo").exists()
            || dir.join("en").join("LC_MESSAGES").join("mirrorman.mo").exists()
        {
            return dir.to_string_lossy().to_string();
        }
    }
    candidates[0].to_string_lossy().to_string()
}

pub fn tr(s: &'static str) -> &'static str {
    let cache = CACHE.read().unwrap();
    if let Some(&v) = cache.get(s) {
        return v;
    }
    drop(cache);
    let translated = gettextrs::gettext(s);
    let leaked: &'static str = Box::leak(translated.into_boxed_str());
    let mut cache = CACHE.write().unwrap();
    cache.insert(s, leaked);
    leaked
}

#[macro_export]
macro_rules! tr {
    ($s:expr) => { $crate::i18n::tr($s) };
}
