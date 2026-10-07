use std::collections::BTreeSet;

pub fn normalize_text(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut prior_space = true;
    for character in text.chars().flat_map(char::to_lowercase) {
        let normalized = match character {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            value if value.is_alphanumeric() || value == '+' || value == '#' => value,
            _ => ' ',
        };
        if normalized == ' ' {
            if !prior_space {
                output.push(' ');
            }
            prior_space = true;
        } else {
            output.push(normalized);
            prior_space = false;
        }
    }
    output.trim().to_string()
}

pub fn query_terms(text: &str) -> BTreeSet<String> {
    let normalized = normalize_text(text);
    let words = normalized.split_whitespace().collect::<Vec<_>>();
    let mut terms = words
        .iter()
        .map(|word| (*word).to_string())
        .collect::<BTreeSet<_>>();
    for width in 2..=words.len().min(5) {
        for window in words.windows(width) {
            terms.insert(window.join(" "));
        }
    }
    terms
}

#[cfg(test)]
mod tests {
    use super::{normalize_text, query_terms};

    #[test]
    fn normalization_is_case_and_punctuation_stable() {
        assert_eq!(
            normalize_text("  Schrödinger's—CAT!  "),
            "schrodinger s cat"
        );
    }

    #[test]
    fn query_terms_include_bounded_phrases() {
        let terms = query_terms("causal inference under confounding");
        assert!(terms.contains("causal inference"));
        assert!(terms.contains("causal inference under confounding"));
    }
}
