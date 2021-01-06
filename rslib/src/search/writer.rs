// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

use crate::{
    backend_proto::concatenate_searches_in::Separator,
    decks::DeckID as DeckIDType,
    err::Result,
    notetype::NoteTypeID as NoteTypeIDType,
    search::parser::{parse, Node, PropertyKind, SearchNode, StateKind, TemplateKind},
};
use itertools::Itertools;

/// Take an Anki-style search string and convert it into an equivalent
/// search string with normalized syntax.
pub fn normalize_search(input: &str) -> Result<String> {
    Ok(write_nodes(&parse(input)?))
}

/// Take an Anki-style search string and return the negated counterpart.
/// Empty searches (whole collection) remain unchanged.
pub fn negate_search(input: &str) -> Result<String> {
    let mut nodes = parse(input)?;
    use Node::*;
    Ok(if nodes.len() == 1 {
        let node = nodes.remove(0);
        match node {
            Not(n) => write_node(&n),
            Search(SearchNode::WholeCollection) => "".to_string(),
            Group(_) | Search(_) => write_node(&Not(Box::new(node))),
            _ => unreachable!(),
        }
    } else {
        write_node(&Not(Box::new(Group(nodes))))
    })
}

/// Take arbitrary Anki-style search strings and return their concatenation where they
/// are separated by the provided boolean operator.
/// Empty searches (whole collection) are left out.
pub fn concatenate_searches(sep: i32, searches: &[String]) -> Result<String> {
    let bool_node = vec![if let Some(Separator::Or) = Separator::from_i32(sep) {
        Node::Or
    } else {
        Node::And
    }];
    Ok(write_nodes(
        searches
            .iter()
            .map(|s: &String| -> Result<Vec<Node>> { parse(s) })
            .collect::<Result<Vec<Vec<Node>>>>()?
            .iter()
            .filter(|v| v[0] != Node::Search(SearchNode::WholeCollection))
            .intersperse(&&bool_node)
            .flat_map(|v| v.iter()),
    ))
}

}

fn write_nodes<'a, I>(nodes: I) -> String
where
    I: IntoIterator<Item = &'a Node<'a>>,
{
    nodes.into_iter().map(|node| write_node(node)).collect()
}

fn write_node(node: &Node) -> String {
    use Node::*;
    match node {
        And => " AND ".to_string(),
        Or => " OR ".to_string(),
        Not(n) => format!("-{}", write_node(n)),
        Group(ns) => format!("({})", write_nodes(ns)),
        Search(n) => write_search_node(n),
    }
}

fn write_search_node(node: &SearchNode) -> String {
    use SearchNode::*;
    match node {
        UnqualifiedText(s) => quote(&s.replace(":", "\\:")),
        SingleField { field, text, is_re } => write_single_field(field, text, *is_re),
        AddedInDays(u) => format!("\"added:{}\"", u),
        EditedInDays(u) => format!("\"edited:{}\"", u),
        CardTemplate(t) => write_template(t),
        Deck(s) => quote(&format!("deck:{}", s)),
        DeckID(DeckIDType(i)) => format!("\"did:{}\"", i),
        NoteTypeID(NoteTypeIDType(i)) => format!("\"mid:{}\"", i),
        NoteType(s) => quote(&format!("note:{}", s)),
        Rated { days, ease } => write_rated(days, ease),
        Tag(s) => quote(&format!("tag:{}", s)),
        Duplicates { note_type_id, text } => quote(&format!("dupes:{},{}", note_type_id, text)),
        State(k) => write_state(k),
        Flag(u) => format!("\"flag:{}\"", u),
        NoteIDs(s) => format!("\"nid:{}\"", s),
        CardIDs(s) => format!("\"cid:{}\"", s),
        Property { operator, kind } => write_property(operator, kind),
        WholeCollection => "".to_string(),
        Regex(s) => quote(&format!("re:{}", s)),
        NoCombining(s) => quote(&format!("nc:{}", s)),
        WordBoundary(s) => quote(&format!("w:{}", s)),
    }
}

/// Escape and wrap in double quotes.
fn quote(txt: &str) -> String {
    format!("\"{}\"", txt.replace("\"", "\\\""))
}

fn write_single_field(field: &str, text: &str, is_re: bool) -> String {
    let re = if is_re { "re:" } else { "" };
    let text = if !is_re && text.starts_with("re:") {
        text.replacen(":", "\\:", 1)
    } else {
        text.to_string()
    };
    quote(&format!("{}:{}{}", field.replace(":", "\\:"), re, &text))
}

fn write_template(template: &TemplateKind) -> String {
    match template {
        TemplateKind::Ordinal(u) => format!("\"card:{}\"", u),
        TemplateKind::Name(s) => format!("\"card:{}\"", s),
    }
}

fn write_rated(days: &u32, ease: &Option<u8>) -> String {
    match ease {
        Some(u) => format!("\"rated:{}:{}\"", days, u),
        None => format!("\"rated:{}\"", days),
    }
}

fn write_state(kind: &StateKind) -> String {
    use StateKind::*;
    format!(
        "\"is:{}\"",
        match kind {
            New => "new",
            Review => "review",
            Learning => "learn",
            Due => "due",
            Buried => "buried",
            UserBuried => "buried-manually",
            SchedBuried => "buried-sibling",
            Suspended => "suspended",
        }
    )
}

fn write_property(operator: &str, kind: &PropertyKind) -> String {
    use PropertyKind::*;
    match kind {
        Due(i) => format!("\"prop:due{}{}\"", operator, i),
        Interval(u) => format!("\"prop:ivl{}{}\"", operator, u),
        Reps(u) => format!("\"prop:reps{}{}\"", operator, u),
        Lapses(u) => format!("\"prop:lapses{}{}\"", operator, u),
        Ease(f) => format!("\"prop:ease{}{}\"", operator, f),
    }
}
