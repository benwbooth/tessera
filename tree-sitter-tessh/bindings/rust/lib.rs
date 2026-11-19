//! tessh's tree-sitter grammar: `LANGUAGE` and its highlight query.

use tree_sitter_language::LanguageFn;

extern "C" {
    fn tree_sitter_tessh() -> *const ();
}

pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_tessh) };

pub const HIGHLIGHTS_QUERY: &str = include_str!("../../queries/highlights.scm");

#[cfg(test)]
mod tests {
    #[test]
    fn loads() {
        let mut p = tree_sitter::Parser::new();
        p.set_language(&super::LANGUAGE.into()).unwrap();
        let t = p.parse("ls -la | grep x\ndef f x\n  print $x\n", None).unwrap();
        assert!(!t.root_node().has_error());
        tree_sitter::Query::new(&super::LANGUAGE.into(), super::HIGHLIGHTS_QUERY).unwrap();
    }
}
