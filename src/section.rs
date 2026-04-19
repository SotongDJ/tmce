//! Section-type tag for a TMC row.
//!
//! Known types are defined by the TMC spec; unrecognised tags are preserved
//! verbatim via `Other(..)` so round-tripping a container never loses data.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionType {
    Header,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    Content,
    Quote,
    Ul,
    Li,
    Codefence,
    Hr,
    Other(String),
}

impl SectionType {
    pub fn parse(s: &str) -> Self {
        match s {
            "header" => Self::Header,
            "h1" => Self::H1,
            "h2" => Self::H2,
            "h3" => Self::H3,
            "h4" => Self::H4,
            "h5" => Self::H5,
            "h6" => Self::H6,
            "content" => Self::Content,
            "quote" => Self::Quote,
            "ul" => Self::Ul,
            "li" => Self::Li,
            "codefence" => Self::Codefence,
            "hr" => Self::Hr,
            other => Self::Other(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Header => "header",
            Self::H1 => "h1",
            Self::H2 => "h2",
            Self::H3 => "h3",
            Self::H4 => "h4",
            Self::H5 => "h5",
            Self::H6 => "h6",
            Self::Content => "content",
            Self::Quote => "quote",
            Self::Ul => "ul",
            Self::Li => "li",
            Self::Codefence => "codefence",
            Self::Hr => "hr",
            Self::Other(s) => s.as_str(),
        }
    }
}

impl fmt::Display for SectionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_round_trip() {
        for s in [
            "header",
            "h1",
            "h2",
            "h3",
            "h4",
            "content",
            "quote",
            "ul",
            "li",
            "codefence",
            "hr",
        ] {
            assert_eq!(SectionType::parse(s).as_str(), s);
        }
    }

    #[test]
    fn unknown_preserved() {
        let t = SectionType::parse("table");
        assert_eq!(t.as_str(), "table");
    }
}
