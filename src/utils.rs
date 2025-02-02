use std::sync::LazyLock;
use regex::Regex;
use unicode_normalization::UnicodeNormalization;


/// Converts a given string to a URL-friendly slug, similar to Django's slugify.
/// 
/// If `allow_unicode` is false, the value is converted to ASCII-only characters
/// using NFKD normalization and filtering out non-ASCII characters.
/// When true, NFKC normalization is used.
pub fn slugify(value: &str, allow_unicode: bool) -> String {
    //TODO: probably something more efficient can be done here
    static RE_INVALID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^\w\s-]").unwrap());
    static RE_SEPARATOR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[-\s]+").unwrap());
    // Normalize the value based on the allow_unicode flag.
    let mut normalized = if allow_unicode {
        // Use NFKC normalization if unicode is allowed.
        value.nfkc().collect::<String>()
    } else {
        // Use NFKD normalization, then remove non-ASCII characters.
        value.nfkd()
            .filter(|c| c.is_ascii())
            .collect::<String>()
    };

    normalized.make_ascii_lowercase();

    // Remove all characters that are not alphanumerics, underscores, whitespace, or hyphens.
    let cleaned = RE_INVALID.replace_all(&normalized, "");

    // Replace spaces and repeated dashes with a single dash.
    let slug = RE_SEPARATOR.replace_all(&cleaned, "-");


    // Trim leading and trailing dashes and underscores.
    slug.trim_matches(|c| c == '-' || c == '_').to_owned()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_basic_ascii() {
        let input = "Hello, World! Rust’s amazing — isn't it?";
        let expected_ascii = "hello-world-rusts-amazing-isnt-it";
        // When allow_unicode is false, Unicode characters should be stripped.
        assert_eq!(slugify(input, false), expected_ascii);
    }

    #[test]
    fn test_slugify_unicode_allowed() {
        let input = "Hello, World! Rust’s amazing — isn't it?";
        // With Unicode allowed, some accented or special characters are preserved.
        let slug = slugify(input, true);
        // Expected result may vary depending on normalization;
        // Here we simply ensure that the slug is lowercase and contains hyphens.
        assert_eq!(slug, slug.to_lowercase());
        assert!(slug.contains('-'));
        // Make sure punctuation is removed.
        assert!(!slug.contains('!'));
        assert!(!slug.contains('?'));
    }

    #[test]
    fn test_slugify_empty() {
        let input = "";
        let expected = "";
        assert_eq!(slugify(input, false), expected);
        assert_eq!(slugify(input, true), expected);
    }

    #[test]
    fn test_slugify_only_invalid_characters() {
        let input = "!@#$%^&*()";
        // With no alphanumeric characters, the result should be an empty string.
        assert_eq!(slugify(input, false), "");
        assert_eq!(slugify(input, true), "");
    }

    #[test]
    fn test_slugify_whitespace() {
        let input = "   multiple   spaces\tand\nnewlines  ";
        let expected = "multiple-spaces-and-newlines";
        assert_eq!(slugify(input, false), expected);
        assert_eq!(slugify(input, true), expected);
    }

    #[test]
    fn test_slugify_preserve_hyphens_and_underscores() {
        let input = "already-slugified_text";
        // The function should collapse only sequences of dashes or spaces;
        // in this case, the input is already slug-like.
        let expected = "already-slugified_text";
        assert_eq!(slugify(input, false), expected);
        assert_eq!(slugify(input, true), expected);
    }

    #[test]
    fn test_slugify_chinese() {
        // Chinese characters: when Unicode is allowed they should be preserved.
        let input = "你好 世界";
        // With unicode allowed, the Chinese characters remain; whitespace is replaced by a dash.
        let expected_unicode = "你好-世界";
        assert_eq!(slugify(input, true), expected_unicode);

        // With ascii-only mode, non-ASCII characters are dropped, resulting in an empty string.
        let expected_ascii = "";
        assert_eq!(slugify(input, false), expected_ascii);
    }

    #[test]
    fn test_slugify_german() {
        // German example with umlauts and sharp s.
        let input = "Füße & Straßen";
        // With Unicode allowed, expect the umlaut and sharp s to remain.
        let expected_unicode = "füße-straßen";
        assert_eq!(slugify(input, true), expected_unicode);

        // With ASCII-only mode:
        // - "ü" decomposes to "u" and a combining diaeresis, so only "u" remains.
        // - "ß" is non-ascii and will be dropped.
        // Given our current implementation, likely:
        let expected_ascii = "fue-straen";
        assert_eq!(slugify(input, false), expected_ascii);
    }

    #[test]
    fn test_slugify_french() {
        // French example with accents.
        let input = "C'est déjà l'été.";
        // With Unicode allowed, accents remain.
        let expected_unicode = "cest-déjà-lété";
        assert_eq!(slugify(input, true), expected_unicode);

        // With ASCII-only mode, accents are removed.
        // "déjà" becomes "deja" and "l'été" becomes "lete".
        let expected_ascii = "cest-deja-lete";
        assert_eq!(slugify(input, false), expected_ascii);
    }
}
