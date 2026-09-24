//! Export formatting.
//!
//! Wrapper semantics ported from ASCIIFlow (`client/export.tsx`), MIT © Lewis Hemens.

use super::glyphs::to_basic;
use super::layer::Layer;
use super::text::layer_to_text;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrapper {
    None,
    Star,
    StarFilled,
    TripleQuotes,
    Hash,
    Slash,
    ThreeSlashes,
    Dash,
    Apostrophe,
    Backticks,
    FourSpaces,
    Semicolon,
}

/// `(variant, id, label)`: the id is what config and the CLI store, the label is
/// what the export popover shows.
pub const WRAPPERS: [(Wrapper, &str, &str); 12] = [
    (Wrapper::None, "none", "none"),
    (Wrapper::Star, "star", "/* */"),
    (Wrapper::StarFilled, "star-filled", "/***/"),
    (Wrapper::TripleQuotes, "triple-quotes", "\"\"\" \"\"\""),
    (Wrapper::Hash, "hash", "# hash"),
    (Wrapper::Slash, "slash", "// slash"),
    (Wrapper::ThreeSlashes, "three-slashes", "/// triple"),
    (Wrapper::Dash, "dash", "-- dash"),
    (Wrapper::Apostrophe, "apostrophe", "' apostrophe"),
    (Wrapper::Backticks, "backticks", "``` backticks"),
    (Wrapper::FourSpaces, "four-spaces", "    indent"),
    (Wrapper::Semicolon, "semicolon", "; semicolon"),
];

fn wrapper_info(w: Wrapper) -> (Wrapper, &'static str, &'static str) {
    *WRAPPERS
        .iter()
        .find(|(id, _, _)| *id == w)
        .expect("every wrapper is listed")
}

pub fn wrapper_id(w: Wrapper) -> &'static str {
    wrapper_info(w).1
}

pub fn is_wrapper(value: &str) -> Option<Wrapper> {
    WRAPPERS
        .iter()
        .find(|(_, id, _)| *id == value)
        .map(|(w, _, _)| *w)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Charset {
    Extended,
    Basic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportConfig {
    pub characters: Charset,
    pub wrapper: Wrapper,
    /// Wrap the result in a Markdown code fence.
    pub fenced: bool,
}

pub const DEFAULT_EXPORT: ExportConfig = ExportConfig {
    characters: Charset::Extended,
    wrapper: Wrapper::None,
    fenced: false,
};

pub fn to_basic_text(text: &str) -> String {
    text.chars().map(to_basic).collect()
}

pub fn apply_export_config(text: &str, config: &ExportConfig) -> String {
    let text = if config.characters == Charset::Basic {
        to_basic_text(text)
    } else {
        text.to_string()
    };
    let mut lines: Vec<String> = text.split('\n').map(|l| l.to_string()).collect();
    let prefix = |lines: &mut Vec<String>, p: &str| {
        for line in lines.iter_mut() {
            line.insert_str(0, p);
        }
    };
    match config.wrapper {
        Wrapper::None => {}
        Wrapper::Star => {
            lines.insert(0, "/*".to_string());
            lines.push(" */".to_string());
        }
        Wrapper::StarFilled => {
            lines = lines.into_iter().map(|l| format!(" * {l}")).collect();
            lines.insert(0, "/*".to_string());
            lines.push(" */".to_string());
        }
        Wrapper::TripleQuotes => {
            lines.insert(
                0,
                if config.characters == Charset::Basic {
                    "\"\"\""
                } else {
                    "u\"\"\""
                }
                .to_string(),
            );
            lines.push("\"\"\"".to_string());
        }
        Wrapper::Hash => prefix(&mut lines, "# "),
        Wrapper::Slash => prefix(&mut lines, "// "),
        Wrapper::ThreeSlashes => prefix(&mut lines, "/// "),
        Wrapper::Dash => prefix(&mut lines, "-- "),
        Wrapper::Apostrophe => prefix(&mut lines, "' "),
        Wrapper::Backticks => {
            lines.insert(0, "```".to_string());
            lines.push("```".to_string());
        }
        Wrapper::FourSpaces => prefix(&mut lines, "    "),
        Wrapper::Semicolon => prefix(&mut lines, "; "),
    }
    if config.fenced {
        lines.insert(0, "```text".to_string());
        lines.push("```".to_string());
    }
    lines.join("\n")
}

/// Drawing text: bounding box of all non-empty cells, trailing spaces trimmed per row.
pub fn export_text(layer: &Layer, config: &ExportConfig) -> String {
    apply_export_config(&layer_to_text(layer, None, true), config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::text::text_to_layer;
    use crate::core::vector::Pos;

    fn drawing() -> crate::core::layer::Layer {
        text_to_layer("┌─┐  \n│ ├─►\n└─┘", Pos::default())
    }

    #[test]
    fn every_wrapper_has_a_golden() {
        let l = drawing();
        let text = |wrapper| {
            export_text(
                &l,
                &ExportConfig {
                    characters: Charset::Extended,
                    wrapper,
                    fenced: false,
                },
            )
        };
        assert_eq!(text(Wrapper::None), "┌─┐\n│ ├─►\n└─┘");
        assert_eq!(text(Wrapper::Star), "/*\n┌─┐\n│ ├─►\n└─┘\n */");
        assert_eq!(
            text(Wrapper::StarFilled),
            "/*\n * ┌─┐\n * │ ├─►\n * └─┘\n */"
        );
        assert_eq!(
            text(Wrapper::TripleQuotes),
            "u\"\"\"\n┌─┐\n│ ├─►\n└─┘\n\"\"\""
        );
        assert_eq!(text(Wrapper::Hash), "# ┌─┐\n# │ ├─►\n# └─┘");
        assert_eq!(text(Wrapper::Backticks), "```\n┌─┐\n│ ├─►\n└─┘\n```");
        assert_eq!(text(Wrapper::FourSpaces), "    ┌─┐\n    │ ├─►\n    └─┘");
    }

    #[test]
    fn basic_charset_and_fence() {
        let l = drawing();
        assert_eq!(
            export_text(
                &l,
                &ExportConfig {
                    characters: Charset::Basic,
                    wrapper: Wrapper::None,
                    fenced: false
                }
            ),
            "+-+\n| +->\n+-+"
        );
        assert_eq!(
            apply_export_config(
                "x",
                &ExportConfig {
                    characters: Charset::Basic,
                    wrapper: Wrapper::TripleQuotes,
                    fenced: false
                }
            ),
            "\"\"\"\nx\n\"\"\""
        );
        assert_eq!(
            export_text(
                &l,
                &ExportConfig {
                    characters: Charset::Basic,
                    wrapper: Wrapper::Hash,
                    fenced: true
                }
            ),
            "```text\n# +-+\n# | +->\n# +-+\n```"
        );
    }

    #[test]
    fn bounding_box_trims_margins() {
        let l = text_to_layer("   \n   a   \n\n     b", Pos::default());
        assert_eq!(export_text(&l, &DEFAULT_EXPORT), "a\n\n  b");
        assert_eq!(
            export_text(&text_to_layer("", Pos::default()), &DEFAULT_EXPORT),
            ""
        );
    }
}
