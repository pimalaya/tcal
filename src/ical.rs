//! # Reading and editing a calendar
//!
//! The one reader every verb uses, and the byte-preserving edits a fold-back
//! makes through it.
//!
//! [`TcalCalendar::parse`] reads a whole stream into ical-rs's syntax tree, which
//! reproduces the wire bytes exactly. A calendar is therefore read once: what
//! the merge reconciles and what the projection walks are the same tree, and
//! no value passes through a second reader that might normalise it.
//!
//! [`TcalComponent`] reads a component and applies the edits a document makes,
//! [`TcalContainer`] counts the children of one, and [`TcalProp`] reads one content
//! line. An unchanged line keeps its own bytes, its layout and its parameter
//! casing included, and only a line the document moved is written anew.

use core::fmt;

use alloc::{
    borrow::{Cow, ToOwned},
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use ical::tree::{
    codec::mode::Escaper,
    cst::{IcalCst, IcalItem},
    leaf::IcalLeaf,
    line::IcalLine,
    param::node::IcalParamNode,
};

use crate::{
    error::{TcalError, TcalResult},
    template::patch::{Content, split},
};

/// A parsed iCalendar stream: the calendars it holds, byte for byte.
///
/// A file usually holds one. Where it holds more, the first is the one every
/// verb reads and the rest are carried through untouched, since a calendar a
/// reader never asked about is not one to drop.
#[derive(Clone, Default)]
pub struct TcalCalendar<'a>(pub Vec<IcalCst<'a>>);

impl<'a> TcalCalendar<'a> {
    /// Parse a whole iCalendar stream.
    ///
    /// A bare component with no `VCALENDAR` around it is accepted as well as a
    /// full calendar.
    pub fn parse(input: &'a str) -> TcalResult<Self> {
        IcalCst::parse_many(input)
            .collect::<core::result::Result<Vec<_>, _>>()
            .map(Self)
            .map_err(|err| TcalError::ParseICalendar(err.to_string()))
    }

    /// The calendar every verb reads, which is the first of the stream.
    pub fn read(&self) -> Option<&IcalCst<'a>> {
        self.0.first()
    }

    /// The top-level components: the `VCALENDAR`'s children, or the lone
    /// component of a bare stream, which has no calendar around it.
    pub fn top_level(&self) -> Vec<&IcalCst<'a>> {
        let Some(root) = self.read() else {
            return Vec::new();
        };

        match root.named("VCALENDAR") {
            true => root.nested().collect(),
            false => vec![root],
        }
    }

    /// The escaping rules the stream's own version is read and written in.
    pub fn escaper(&self) -> Escaper {
        self.read()
            .map(|root| Escaper::for_version(root.version()))
            .unwrap_or_default()
    }
}

impl<'a> From<IcalCst<'a>> for TcalCalendar<'a> {
    fn from(cst: IcalCst<'a>) -> Self {
        TcalCalendar(vec![cst])
    }
}

impl fmt::Display for TcalCalendar<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for cst in &self.0 {
            cst.fmt(f)?;
        }

        Ok(())
    }
}

/// Whatever holds direct child components.
///
/// That is one calendar, or the stream of them a bare component sits loose
/// in.
pub trait TcalContainer<'a> {
    /// The direct child components of that name, in source order.
    fn children_mut<'s>(&'s mut self, name: &str) -> impl Iterator<Item = &'s mut IcalCst<'a>>
    where
        'a: 's;

    /// Make them number exactly `count`.
    ///
    /// Empty ones are appended, and a surplus is dropped from the back so the
    /// ones before it keep their bytes.
    fn set_child_count(&mut self, name: &str, count: usize);
}

/// The reading of one component, and the byte-preserving edits a fold-back
/// makes to it.
pub trait TcalComponent<'a>: TcalContainer<'a> {
    /// Whether the component carries that name.
    ///
    /// iCalendar compares a name without regard to case (RFC 5545 section
    /// 3.1).
    fn named(&self, name: &str) -> bool;

    /// The direct properties of that name, in source order.
    fn props<'s>(&'s self, name: &str) -> impl Iterator<Item = &'s IcalLine<'a>>
    where
        'a: 's;

    /// Every direct child component, whatever its name, in source order.
    fn nested<'s>(&'s self) -> impl Iterator<Item = &'s IcalCst<'a>>
    where
        'a: 's;

    /// The direct child components of that name, in source order.
    fn children<'s>(&'s self, name: &str) -> impl Iterator<Item = &'s IcalCst<'a>>
    where
        'a: 's;

    /// The logical content lines of the direct properties of that name.
    ///
    /// They come in source order, each without its line ending.
    fn lines(&self, name: &str) -> Vec<String>;

    /// Make those properties exactly `lines`.
    ///
    /// An unchanged one keeps its own bytes, a surplus one is dropped, and a
    /// missing one is inserted after the last property. An empty slice removes
    /// them all. A line written anew is stamped with `escaper`, the rules the
    /// calendar's own version is read and written in.
    fn set_lines(&mut self, name: &str, lines: &[String], escaper: Escaper);
}

/// The reading of one content line.
pub trait TcalProp {
    /// The logical content line a property occupies.
    ///
    /// That is its name, parameters and value, without the line ending or the
    /// folds it was written with.
    fn logical(&self) -> String;

    /// The value it carries, still escaped.
    ///
    /// Everything after the colon ending the name and parameters, one inside a
    /// quoted parameter value not counting. A structured value (a recurrence
    /// rule) is read in this form, its separators being its own syntax.
    fn raw(&self) -> String;

    /// The value as one unescaped string, its commas kept literal.
    ///
    /// A single-valued property is one value however it is punctuated, so a
    /// comma inside a URI or an unescaped one inside a summary stays in the
    /// value rather than truncating it.
    fn text(&self) -> String;

    /// The value as its comma-separated items, each unescaped on its own.
    fn items(&self) -> Vec<String>;

    /// The first value of a named parameter, unquoted (RFC 5545 section 3.2).
    ///
    /// Named for the value rather than the parameter, ical-rs giving `param`
    /// to the typed lens that reads one.
    fn param_value(&self, name: &str) -> Option<String>;
}

impl<'a> TcalContainer<'a> for TcalCalendar<'a> {
    fn children_mut<'s>(&'s mut self, name: &str) -> impl Iterator<Item = &'s mut IcalCst<'a>>
    where
        'a: 's,
    {
        self.0.iter_mut().filter(move |cst| cst.named(name))
    }

    fn set_child_count(&mut self, name: &str, count: usize) {
        let eol = self.0.first().map(eol_of).unwrap_or_else(crlf);
        let held: Vec<usize> = self
            .0
            .iter()
            .enumerate()
            .filter(|(_, cst)| cst.named(name))
            .map(|(at, _)| at)
            .collect();

        for _ in held.len()..count {
            self.0.push(empty(name, &eol));
        }

        for at in held.iter().skip(count).rev() {
            self.0.remove(*at);
        }
    }
}

impl<'a> TcalContainer<'a> for IcalCst<'a> {
    fn children_mut<'s>(&'s mut self, name: &str) -> impl Iterator<Item = &'s mut IcalCst<'a>>
    where
        'a: 's,
    {
        self.items.iter_mut().filter_map(move |item| match item {
            IcalItem::Component(child) if child.named(name) => Some(&mut **child),
            _ => None,
        })
    }

    fn set_child_count(&mut self, name: &str, count: usize) {
        let eol = eol_of(self);
        let held: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, IcalItem::Component(child) if child.named(name)))
            .map(|(at, _)| at)
            .collect();

        for _ in held.len()..count {
            self.items
                .push(IcalItem::Component(Box::new(empty(name, &eol))));
        }

        for at in held.iter().skip(count).rev() {
            self.items.remove(*at);
        }
    }
}

impl<'a> TcalComponent<'a> for IcalCst<'a> {
    fn named(&self, name: &str) -> bool {
        self.begin
            .as_ref()
            .is_some_and(|begin| begin.raw_value_str().eq_ignore_ascii_case(name))
    }

    fn props<'s>(&'s self, name: &str) -> impl Iterator<Item = &'s IcalLine<'a>>
    where
        'a: 's,
    {
        self.items.iter().filter_map(move |item| match item {
            IcalItem::Prop(line) if line.name.get().eq_ignore_ascii_case(name) => Some(line),
            _ => None,
        })
    }

    fn nested<'s>(&'s self) -> impl Iterator<Item = &'s IcalCst<'a>>
    where
        'a: 's,
    {
        self.items.iter().filter_map(|item| match item {
            IcalItem::Component(child) => Some(&**child),
            _ => None,
        })
    }

    fn children<'s>(&'s self, name: &str) -> impl Iterator<Item = &'s IcalCst<'a>>
    where
        'a: 's,
    {
        self.nested().filter(move |child| child.named(name))
    }

    fn lines(&self, name: &str) -> Vec<String> {
        self.props(name).map(TcalProp::logical).collect()
    }

    fn set_lines(&mut self, name: &str, lines: &[String], escaper: Escaper) {
        let eol = eol_of(self);
        let held: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                matches!(item, IcalItem::Prop(line) if line.name.get().eq_ignore_ascii_case(name))
            })
            .map(|(at, _)| at)
            .collect();

        for (slot, line) in held.iter().zip(lines) {
            let IcalItem::Prop(held) = &mut self.items[*slot] else {
                continue;
            };

            if &held.logical() != line {
                *held = built(line, &eol, escaper);
            }
        }

        for slot in held.iter().skip(lines.len()).rev() {
            self.items.remove(*slot);
        }

        let extra = lines.iter().skip(held.len());

        for (at, line) in (insertion_point(self)..).zip(extra) {
            self.items
                .insert(at, IcalItem::Prop(built(line, &eol, escaper)));
        }
    }
}

impl TcalProp for IcalLine<'_> {
    fn logical(&self) -> String {
        let mut out = String::from(self.name.get());

        for param in &self.params {
            out.push(';');
            out.push_str(&param.to_string());
        }

        out.push(':');
        out.push_str(&self.value.to_string());
        out
    }

    fn raw(&self) -> String {
        Content(&self.logical()).value().to_owned()
    }

    fn text(&self) -> String {
        Content(&self.logical()).text()
    }

    fn items(&self) -> Vec<String> {
        Content(&self.logical()).texts()
    }

    fn param_value(&self, name: &str) -> Option<String> {
        let logical = self.logical();

        split(Content(&logical).head(), ';')
            .into_iter()
            .skip(1)
            .find_map(|param| {
                let (held, value) = param.split_once('=')?;

                held.eq_ignore_ascii_case(name)
                    .then(|| value.trim_matches('"').to_owned())
            })
    }
}

/// Build an owned content line from its text, ended with `eol`.
///
/// The name, every parameter and the value are carried across verbatim, so a
/// parameter a fold-back kept from the source keeps the bytes it had. Both the
/// value and the parameters are stamped with `escaper`.
///
/// The line carries no wire layout, so it goes out unfolded. It is one the
/// document wrote, and a layout is offsets into the bytes of the line it
/// replaces, which are not these.
fn built(text: &str, eol: &str, escaper: Escaper) -> IcalLine<'static> {
    let content = Content(text);
    let mut params = split(content.head(), ';');
    let name = params.remove(0);

    let mut line = IcalLine::text(name.to_owned(), content.value().to_owned());

    line.params = params.iter().map(|text| param(text, escaper)).collect();
    line.value.escaper = escaper;
    line.eol = IcalLeaf(Cow::Owned(eol.to_owned()));
    line
}

/// Build one parameter node from its text, its values left as they were
/// written.
///
/// ical-rs splits the values, a comma inside a quoted one not counting, and
/// the pieces are taken over owned since the text they come from is the line a
/// fold-back has just assembled.
fn param(text: &str, escaper: Escaper) -> IcalParamNode<'static> {
    let node = IcalParamNode::parse(text);

    IcalParamNode {
        name: IcalLeaf(Cow::Owned(node.name.get().to_owned())),
        values: node
            .values
            .iter()
            .map(|value| IcalLeaf(Cow::Owned(value.get().to_owned())))
            .collect(),
        escaper,
    }
}

/// An empty component, its envelope written with the given line ending.
///
/// Its two lines carry neither a parameter nor a value to escape, so they take
/// the default escaping rules.
fn empty(name: &str, eol: &str) -> IcalCst<'static> {
    let escaper = Escaper::default();

    IcalCst {
        begin: Some(built(&format!("BEGIN:{name}"), eol, escaper)),
        items: Vec::new(),
        end: Some(built(&format!("END:{name}"), eol, escaper)),
        trailing: Cow::Borrowed(""),
    }
}

/// The line ending a component was written with, CRLF where it has none.
fn eol_of(component: &IcalCst<'_>) -> String {
    component
        .begin
        .as_ref()
        .map(|begin| begin.eol.get().to_owned())
        .filter(|eol| !eol.is_empty())
        .unwrap_or_else(crlf)
}

/// The line ending assumed where a calendar carries none.
fn crlf() -> String {
    "\r\n".to_owned()
}

/// Where a new property lands.
///
/// After the last property already there, else before the first nested
/// component, else at the end.
fn insertion_point(component: &IcalCst<'_>) -> usize {
    if let Some(last) = component
        .items
        .iter()
        .rposition(|item| matches!(item, IcalItem::Prop(_)))
    {
        return last + 1;
    }

    component
        .items
        .iter()
        .position(|item| matches!(item, IcalItem::Component(_)))
        .unwrap_or(component.items.len())
}
