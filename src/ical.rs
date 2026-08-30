//! # Reading and editing a calendar
//!
//! The one reader every verb uses, and the byte-preserving edits a fold-back
//! makes through it.
//!
//! [`parse`] reads a whole stream into ical-rs's syntax tree, which reproduces
//! the wire bytes exactly. A calendar is therefore read once: what the merge
//! reconciles and what the projection walks are the same tree, and no value
//! passes through a second reader that might normalise it.
//!
//! [`Component`] and [`Container`] are the edits applying a document makes:
//! setting a property's lines and counting a component's children. An
//! unchanged line keeps its own bytes, folds and parameter casing included,
//! and only a line the document actually moved is written anew.

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
    error::{Result, TcalError},
    template::patch::{head, split, value_of},
};

/// A parsed iCalendar stream: the calendars it holds, byte for byte.
///
/// A file usually holds one. Where it holds more, the first is the one every
/// verb reads and the rest are carried through untouched, since a calendar a
/// reader never asked about is not one to drop.
#[derive(Default)]
pub struct Calendar<'a>(pub Vec<IcalCst<'a>>);

impl<'a> Calendar<'a> {
    /// The calendar every verb reads, which is the first of the stream.
    pub fn read(&self) -> Option<&IcalCst<'a>> {
        self.0.first()
    }
}

impl<'a> From<IcalCst<'a>> for Calendar<'a> {
    fn from(cst: IcalCst<'a>) -> Self {
        Calendar(vec![cst])
    }
}

impl fmt::Display for Calendar<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for cst in &self.0 {
            cst.fmt(f)?;
        }

        Ok(())
    }
}

/// Parse a whole iCalendar stream.
///
/// A bare component with no `VCALENDAR` around it is accepted as well as a
/// full calendar.
pub fn parse(input: &str) -> Result<Calendar<'_>> {
    IcalCst::parse_many(input)
        .collect::<core::result::Result<Vec<_>, _>>()
        .map(Calendar)
        .map_err(|err| TcalError::ParseICalendar(err.to_string()))
}

/// Whatever holds direct child components.
///
/// That is one calendar, or the stream of them a bare component sits loose
/// in.
pub trait Container<'a> {
    /// The direct child components of that name, in source order.
    fn children<'s>(&'s mut self, name: &str) -> impl Iterator<Item = &'s mut IcalCst<'a>>
    where
        'a: 's;

    /// Make them number exactly `count`.
    ///
    /// Empty ones are appended, and a surplus is dropped from the back so the
    /// ones before it keep their bytes.
    fn set_child_count(&mut self, name: &str, count: usize);
}

/// The byte-preserving property edits a fold-back makes to one component.
pub trait Component<'a>: Container<'a> {
    /// The logical content lines of the direct properties of that name.
    ///
    /// They come in source order, each without its line ending.
    fn lines(&self, name: &str) -> Vec<String>;

    /// Make those properties exactly `lines`.
    ///
    /// An unchanged one keeps its own bytes, a surplus one is dropped, and a
    /// missing one is inserted after the last property. An empty slice
    /// removes them all.
    fn set_lines(&mut self, name: &str, lines: &[String]);
}

impl<'a> Container<'a> for Calendar<'a> {
    fn children<'s>(&'s mut self, name: &str) -> impl Iterator<Item = &'s mut IcalCst<'a>>
    where
        'a: 's,
    {
        self.0.iter_mut().filter(move |cst| named(cst, name))
    }

    fn set_child_count(&mut self, name: &str, count: usize) {
        let eol = self.0.first().map(eol_of).unwrap_or_else(crlf);
        let held: Vec<usize> = self
            .0
            .iter()
            .enumerate()
            .filter(|(_, cst)| named(cst, name))
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

impl<'a> Container<'a> for IcalCst<'a> {
    fn children<'s>(&'s mut self, name: &str) -> impl Iterator<Item = &'s mut IcalCst<'a>>
    where
        'a: 's,
    {
        self.items.iter_mut().filter_map(move |item| match item {
            IcalItem::Component(child) if named(child, name) => Some(&mut **child),
            _ => None,
        })
    }

    fn set_child_count(&mut self, name: &str, count: usize) {
        let eol = eol_of(self);
        let held: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, IcalItem::Component(child) if named(child, name)))
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

impl<'a> Component<'a> for IcalCst<'a> {
    fn lines(&self, name: &str) -> Vec<String> {
        props(self, name).map(logical).collect()
    }

    fn set_lines(&mut self, name: &str, lines: &[String]) {
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

            // NOTE: a line the document did not move keeps its own bytes,
            // where the folds and parameter casing of the source live.
            if &logical(held) != line {
                *held = built(line, &eol);
            }
        }

        for slot in held.iter().skip(lines.len()).rev() {
            self.items.remove(*slot);
        }

        let extra = lines.iter().skip(held.len());

        for (at, line) in (insertion_point(self)..).zip(extra) {
            self.items.insert(at, IcalItem::Prop(built(line, &eol)));
        }
    }
}

/// The direct properties of that name, in source order.
pub fn props<'c, 'a>(
    component: &'c IcalCst<'a>,
    name: &str,
) -> impl Iterator<Item = &'c IcalLine<'a>> {
    component.items.iter().filter_map(move |item| match item {
        IcalItem::Prop(line) if line.name.get().eq_ignore_ascii_case(name) => Some(line),
        _ => None,
    })
}

/// Every direct child component, whatever its name, in source order.
pub fn nested<'c, 'a>(component: &'c IcalCst<'a>) -> impl Iterator<Item = &'c IcalCst<'a>> {
    component.items.iter().filter_map(|item| match item {
        IcalItem::Component(child) => Some(&**child),
        _ => None,
    })
}

/// The direct child components of that name, in source order.
pub fn children<'c, 'a>(
    component: &'c IcalCst<'a>,
    name: &str,
) -> impl Iterator<Item = &'c IcalCst<'a>> {
    nested(component).filter(move |child| named(child, name))
}

/// Whether a component carries that name.
///
/// iCalendar compares a name without regard to case (RFC 5545 section 3.1).
pub fn named(component: &IcalCst<'_>, name: &str) -> bool {
    component
        .begin
        .as_ref()
        .is_some_and(|begin| begin.raw_value_str().eq_ignore_ascii_case(name))
}

/// The logical content line a property occupies.
///
/// That is its name, parameters and value, without the line ending or the
/// folds it was written with.
pub fn logical(line: &IcalLine<'_>) -> String {
    let mut out = String::from(line.name.get());

    for param in &line.params {
        out.push(';');
        out.push_str(&param.to_string());
    }

    out.push(':');
    out.push_str(&line.value.to_string());
    out
}

/// Build an owned content line from its text, ended with `eol`.
///
/// The name, every parameter and the value are carried across verbatim, so a
/// parameter a fold-back kept from the source keeps the bytes it had.
fn built(text: &str, eol: &str) -> IcalLine<'static> {
    let mut params = split(head(text), ';');
    let name = params.remove(0);

    let mut line = IcalLine::text(name.to_owned(), value_of(text).to_owned());

    line.params = params.into_iter().map(param).collect();
    line.eol = IcalLeaf(Cow::Owned(eol.to_owned()));
    line
}

/// Build one parameter node from its text, its value left as it was written.
fn param(text: &str) -> IcalParamNode<'static> {
    let (name, values) = match text.split_once('=') {
        Some((name, values)) => (name, vec![IcalLeaf(Cow::Owned(values.to_owned()))]),
        None => (text, Vec::new()),
    };

    IcalParamNode {
        name: IcalLeaf(Cow::Owned(name.to_owned())),
        values,
        escaper: Escaper::default(),
    }
}

/// An empty component, its envelope written with the given line ending.
fn empty(name: &str, eol: &str) -> IcalCst<'static> {
    IcalCst {
        begin: Some(built(&format!("BEGIN:{name}"), eol)),
        items: Vec::new(),
        end: Some(built(&format!("END:{name}"), eol)),
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
