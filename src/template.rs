//! Projection between a calcard [`VCard`] and an ergonomic TOML
//! buffer.
//!
//! [`project`] turns a vCard into a fillable TOML form: known fields
//! are prefilled, the rest are listed empty (an empty value means the
//! same as a removed line, so nothing is commented out). A hint, when
//! useful, sits inline next to the value, and hints within a block are
//! aligned to a common column. [`apply`] takes the original vCard plus
//! the edited buffer and produces an updated vCard, rebuilding the
//! modeled fields from TOML while carrying every unmodeled property
//! (custom `X-*`, vendor extensions, ...) verbatim.
//!
//! The buffer is an editing affordance, not an interchange format:
//! `apply` always needs the original vCard, because that is where
//! unmodeled properties live.
//!
//! NOTE: TOML attributes every bare key after a `[table]` / `[[array]]`
//! header to that table, so [`FIELDS`] lists all scalar/list keys
//! first and every sectioned property (`N`, `EMAIL`, `ADR`, ...) last.

use std::fmt::Write as _;

use calcard::{
    common::IanaString,
    vcard::{
        VCard, VCardEntry, VCardParameterName, VCardParameterValue, VCardProperty, VCardValue,
        VCardVersion,
    },
};
use toml_edit::{DocumentMut, Item, TableLike};

use crate::error::{Result, TcardError};

/// Project a vCard into a fillable TOML form.
///
/// An empty [`VCard`] yields a blank template: `uid` first, then the
/// rest of the bare keys as one block, sections last.
pub fn project(vcard: &VCard, version: VCardVersion) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "# vCard {version} as TOML, edited by tcard.");
    let _ = writeln!(out, "#");
    let _ = writeln!(
        out,
        "# Fill what you need; empty fields are ignored. Properties"
    );
    let _ = writeln!(
        out,
        "# tcard does not model are kept verbatim, not shown here."
    );

    let collect = |field: &Field| -> Vec<&VCardEntry> {
        vcard
            .entries
            .iter()
            .filter(|entry| entry.name.as_str() == field.name)
            .collect()
    };

    // uid leads, set off by a blank line above and below.
    let _ = writeln!(out);
    emit_lines(&mut out, &FIELDS[0].lines(&collect(&FIELDS[0])), 0);
    let _ = writeln!(out);

    // The remaining bare keys form one block with a shared comment
    // column.
    let bare: Vec<&Field> = FIELDS[1..]
        .iter()
        .take_while(|field| field.kind.is_simple())
        .collect();
    let bare_lines: Vec<Line> = bare
        .iter()
        .flat_map(|field| field.lines(&collect(field)))
        .collect();
    emit_lines(&mut out, &bare_lines, comment_column(bare_lines.iter()));

    // Each section is set off by a blank line and aligned within itself.
    for field in &FIELDS[1 + bare.len()..] {
        let _ = writeln!(out);
        let lines = field.lines(&collect(field));
        emit_lines(&mut out, &lines, comment_column(lines.iter()));
    }

    out
}

/// Apply an edited TOML buffer onto the original vCard.
///
/// Modeled fields are rebuilt from the buffer; unmodeled properties
/// of `original` are preserved verbatim. The result is serialized by
/// calcard at the requested `version`, so output is normalized (line
/// folding, parameter casing) but lossless for unknown properties.
pub fn apply(original: &VCard, edited_toml: &str, version: VCardVersion) -> Result<String> {
    let doc: DocumentMut = edited_toml.parse().map_err(TcardError::ParseToml)?;

    let mut assembled = String::from("BEGIN:VCARD\r\n");
    let _ = write!(assembled, "VERSION:{version}\r\n");

    for field in FIELDS {
        field.emit(&doc, &mut assembled);
    }

    assembled.push_str("END:VCARD\r\n");

    let mut rebuilt = crate::vcard::parse(&assembled)?;
    rebuilt.entries.retain(|entry| is_data(&entry.name));

    for entry in &original.entries {
        if is_data(&entry.name) && !is_modeled(&entry.name) {
            rebuilt.entries.push(entry.clone());
        }
    }

    let mut out = String::new();
    rebuilt
        .write_to(&mut out, version)
        .expect("writing a vCard to a String is infallible");

    Ok(out)
}

/// A projected line: a left side and an optional inline hint.
struct Line {
    lhs: String,
    hint: Option<String>,
}

/// Shape of a modeled property, driving both projection and emission.
///
/// `TYPE` never changes a property's shape (an `EMAIL` is one value
/// whether home or work), so typed properties keep a single section
/// and list their accepted types in a trailing comment.
enum Kind {
    /// Single text value (`FN`, `UID`, ...).
    Scalar,

    /// Free-form text, projected as a TOML multi-line literal (`NOTE`).
    Text,

    /// Repeated or multi-valued text, joined on `sep` in the vCard
    /// (`NICKNAME`, `CATEGORIES`, `ORG`).
    List { sep: char },

    /// One structured value with named, ordered components (`N`).
    Structured(&'static [&'static str]),

    /// Repeatable property with an optional `TYPE` and a single value
    /// (`EMAIL`, `TEL`, `URL`, `PHOTO`).
    Typed { types: &'static [&'static str] },

    /// Repeatable property with an optional `TYPE` and named, ordered
    /// components (`ADR`).
    TypedStructured {
        types: &'static [&'static str],
        components: &'static [&'static str],
    },
}

impl Kind {
    /// A bare key (vs a `[table]` / `[[array]]` section).
    fn is_simple(&self) -> bool {
        matches!(self, Kind::Scalar | Kind::Text | Kind::List { .. })
    }
}

/// A modeled vCard property and how it maps to TOML.
struct Field {
    /// TOML key.
    key: &'static str,

    /// Canonical vCard property name (matches calcard's `as_str`).
    name: &'static str,

    /// Inline hint shown next to the value, only where it is not
    /// self-evident (rendered as `  # <hint>`).
    hint: Option<&'static str>,

    /// Mapping shape.
    kind: Kind,
}

/// `N` components, in RFC 6350 order.
const NAME_COMPONENTS: &[&str] = &["family", "given", "additional", "prefixes", "suffixes"];

/// `ADR` components, in RFC 6350 order.
const ADR_COMPONENTS: &[&str] = &[
    "pobox", "ext", "street", "locality", "region", "code", "country",
];

/// Common `TYPE` sets, shared between properties.
const PLACE_TYPES: &[&str] = &["home", "work"];
const TEL_TYPES: &[&str] = &[
    "home",
    "work",
    "cell",
    "fax",
    "voice",
    "video",
    "pager",
    "text",
    "textphone",
];

/// The modeled vocabulary. Everything outside this list is preserved
/// verbatim by [`apply`] but not surfaced in the scaffold.
///
/// `uid` leads, the remaining bare keys follow as one block (`note`
/// last, so its literal block sits at the end), and the sectioned
/// properties come last: a TOML document root ends at the first table
/// or array-of-tables header.
const FIELDS: &[Field] = &[
    Field {
        key: "uid",
        name: "UID",
        hint: None,
        kind: Kind::Scalar,
    },
    Field {
        key: "fn",
        name: "FN",
        hint: Some("required"),
        kind: Kind::Scalar,
    },
    Field {
        key: "kind",
        name: "KIND",
        hint: Some("e.g. individual, group, org"),
        kind: Kind::Scalar,
    },
    Field {
        key: "nickname",
        name: "NICKNAME",
        hint: None,
        kind: Kind::List { sep: ',' },
    },
    Field {
        key: "org",
        name: "ORG",
        hint: None,
        kind: Kind::List { sep: ';' },
    },
    Field {
        key: "title",
        name: "TITLE",
        hint: None,
        kind: Kind::Scalar,
    },
    Field {
        key: "role",
        name: "ROLE",
        hint: None,
        kind: Kind::Scalar,
    },
    Field {
        key: "categories",
        name: "CATEGORIES",
        hint: None,
        kind: Kind::List { sep: ',' },
    },
    Field {
        key: "lang",
        name: "LANG",
        hint: None,
        kind: Kind::List { sep: ',' },
    },
    Field {
        key: "bday",
        name: "BDAY",
        hint: Some("e.g. 1990-05-23"),
        kind: Kind::Scalar,
    },
    Field {
        key: "anniversary",
        name: "ANNIVERSARY",
        hint: Some("e.g. 2014-09-21"),
        kind: Kind::Scalar,
    },
    Field {
        key: "geo",
        name: "GEO",
        hint: Some("e.g. geo:37.78,-122.40"),
        kind: Kind::Scalar,
    },
    Field {
        key: "tz",
        name: "TZ",
        hint: Some("e.g. America/New_York"),
        kind: Kind::Scalar,
    },
    Field {
        key: "note",
        name: "NOTE",
        hint: None,
        kind: Kind::Text,
    },
    Field {
        key: "name",
        name: "N",
        hint: None,
        kind: Kind::Structured(NAME_COMPONENTS),
    },
    Field {
        key: "email",
        name: "EMAIL",
        hint: None,
        kind: Kind::Typed { types: PLACE_TYPES },
    },
    Field {
        key: "tel",
        name: "TEL",
        hint: None,
        kind: Kind::Typed { types: TEL_TYPES },
    },
    Field {
        key: "address",
        name: "ADR",
        hint: None,
        kind: Kind::TypedStructured {
            types: PLACE_TYPES,
            components: ADR_COMPONENTS,
        },
    },
    Field {
        key: "photo",
        name: "PHOTO",
        hint: Some("e.g. file:// or http://"),
        kind: Kind::Typed { types: &[] },
    },
    Field {
        key: "url",
        name: "URL",
        hint: None,
        kind: Kind::Typed { types: PLACE_TYPES },
    },
    Field {
        key: "impp",
        name: "IMPP",
        hint: Some("e.g. xmpp:jane@example.com"),
        kind: Kind::Typed { types: PLACE_TYPES },
    },
];

impl Field {
    /// Render this field into projected lines.
    fn lines(&self, entries: &[&VCardEntry]) -> Vec<Line> {
        match &self.kind {
            Kind::Scalar => {
                let value = entries
                    .first()
                    .and_then(|entry| entry_text(entry))
                    .unwrap_or_default();
                vec![Line {
                    lhs: format!("{} = {}", self.key, toml_str(value)),
                    hint: self.hint.map(str::to_owned),
                }]
            }

            Kind::Text => {
                let value = entries
                    .first()
                    .and_then(|entry| entry_text(entry))
                    .unwrap_or_default();
                text_lines(self.key, value)
            }

            Kind::List { .. } => {
                let items: Vec<String> = entries
                    .iter()
                    .flat_map(|entry| entry_texts(entry))
                    .collect();
                vec![Line {
                    lhs: format!("{} = {}", self.key, toml_array(&items)),
                    hint: self.hint.map(str::to_owned),
                }]
            }

            Kind::Structured(components) => {
                let values = entries
                    .first()
                    .map(|entry| entry_components(entry))
                    .unwrap_or_default();
                let mut lines = vec![Line {
                    lhs: format!("[{}]", self.key),
                    hint: None,
                }];
                lines.extend(component_lines(components, &values));
                lines
            }

            Kind::Typed { types } => {
                let mut lines = Vec::new();

                if entries.is_empty() {
                    lines.push(Line {
                        lhs: format!("[[{}]]", self.key),
                        hint: None,
                    });
                    type_line(&mut lines, "", types);
                    lines.push(Line {
                        lhs: "value = \"\"".into(),
                        hint: self.hint.map(str::to_owned),
                    });
                } else {
                    for entry in entries {
                        lines.push(Line {
                            lhs: format!("[[{}]]", self.key),
                            hint: None,
                        });
                        type_line(&mut lines, &type_strings(entry).join(","), types);
                        let value = entry_text(entry).unwrap_or_default();
                        lines.push(Line {
                            lhs: format!("value = {}", toml_str(value)),
                            hint: self.hint.map(str::to_owned),
                        });
                    }
                }

                lines
            }

            Kind::TypedStructured { types, components } => {
                let mut lines = Vec::new();

                if entries.is_empty() {
                    lines.push(Line {
                        lhs: format!("[[{}]]", self.key),
                        hint: None,
                    });
                    type_line(&mut lines, "", types);
                    lines.extend(component_lines(components, &[]));
                } else {
                    for entry in entries {
                        lines.push(Line {
                            lhs: format!("[[{}]]", self.key),
                            hint: None,
                        });
                        type_line(&mut lines, &type_strings(entry).join(","), types);
                        lines.extend(component_lines(components, &entry_components(entry)));
                    }
                }

                lines
            }
        }
    }

    /// Emit this field's vCard content line(s) from the edited `doc`
    /// into `out`, skipping empty values.
    fn emit(&self, doc: &DocumentMut, out: &mut String) {
        let Some(item) = doc.get(self.key) else {
            return;
        };

        match &self.kind {
            Kind::Scalar => {
                if let Some(value) = item.as_str().filter(|value| !value.is_empty()) {
                    push_line(out, &format!("{}:{}", self.name, escape(value)));
                }
            }

            Kind::Text => {
                // Drop the trailing newline the literal block adds.
                if let Some(value) = item.as_str() {
                    let value = value.strip_suffix('\n').unwrap_or(value);
                    if !value.is_empty() {
                        push_line(out, &format!("{}:{}", self.name, escape(value)));
                    }
                }
            }

            Kind::List { sep } => {
                let Some(array) = item.as_array() else {
                    return;
                };

                let parts: Vec<String> = array
                    .iter()
                    .filter_map(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                    .map(escape)
                    .collect();

                if !parts.is_empty() {
                    push_line(
                        out,
                        &format!("{}:{}", self.name, parts.join(&sep.to_string())),
                    );
                }
            }

            Kind::Structured(components) => {
                let Some(table) = item.as_table_like() else {
                    return;
                };

                let parts = read_components(table, components);

                if parts.iter().any(|part| !part.is_empty()) {
                    push_line(out, &format!("{}:{}", self.name, parts.join(";")));
                }
            }

            Kind::Typed { .. } => {
                for table in tables(item) {
                    let Some(value) = table
                        .get("value")
                        .and_then(|item| item.as_str())
                        .filter(|value| !value.is_empty())
                    else {
                        continue;
                    };

                    let mut line = self.name.to_string();
                    push_type(&mut line, table);
                    line.push(':');
                    line.push_str(&escape(value));
                    push_line(out, &line);
                }
            }

            Kind::TypedStructured { components, .. } => {
                for table in tables(item) {
                    let parts = read_components(table, components);

                    if !parts.iter().any(|part| !part.is_empty()) {
                        continue;
                    }

                    let mut line = self.name.to_string();
                    push_type(&mut line, table);
                    line.push(':');
                    line.push_str(&parts.join(";"));
                    push_line(out, &line);
                }
            }
        }
    }
}

/// The column at which a block's inline `#` comments are aligned: the
/// widest left side among the lines that carry a hint.
fn comment_column<'a>(lines: impl Iterator<Item = &'a Line>) -> usize {
    lines
        .filter(|line| line.hint.is_some())
        .map(|line| line.lhs.len())
        .max()
        .unwrap_or(0)
}

/// Emit lines, padding hinted ones so their `#` lands on `column`.
fn emit_lines(out: &mut String, lines: &[Line], column: usize) {
    for line in lines {
        match &line.hint {
            Some(hint) => {
                let _ = writeln!(out, "{:<width$}  # {hint}", line.lhs, width = column);
            }
            None => {
                let _ = writeln!(out, "{}", line.lhs);
            }
        }
    }
}

/// Project a `note` as a TOML multi-line literal: `''''''` when empty,
/// a `'''` block otherwise. Literal strings cannot contain `'''`, so
/// such a value falls back to a basic string.
fn text_lines(key: &str, value: &str) -> Vec<Line> {
    if value.is_empty() {
        return vec![Line {
            lhs: format!("{key} = ''''''"),
            hint: None,
        }];
    }

    if value.contains("'''") {
        return vec![Line {
            lhs: format!("{key} = {}", toml_str(value)),
            hint: None,
        }];
    }

    let mut lines = vec![Line {
        lhs: format!("{key} = '''"),
        hint: None,
    }];
    for content in value.lines() {
        lines.push(Line {
            lhs: content.to_owned(),
            hint: None,
        });
    }
    lines.push(Line {
        lhs: "'''".into(),
        hint: None,
    });
    lines
}

/// Push a `type =` line with its accepted-types hint, when the
/// property has a common type set.
fn type_line(lines: &mut Vec<Line>, value: &str, types: &[&str]) {
    if types.is_empty() {
        return;
    }

    lines.push(Line {
        lhs: format!("type = {}", toml_str(value)),
        hint: Some(format!("e.g. {}", types.join(" "))),
    });
}

/// Render named components, filled or empty, in order.
fn component_lines(components: &[&str], values: &[String]) -> Vec<Line> {
    components
        .iter()
        .enumerate()
        .map(|(index, component)| {
            let value = values.get(index).map(String::as_str).unwrap_or_default();
            Line {
                lhs: format!("{component} = {}", toml_str(value)),
                hint: None,
            }
        })
        .collect()
}

/// Read named components from a TOML table, escaped and in order;
/// missing components become empty strings.
fn read_components(table: &dyn TableLike, components: &[&str]) -> Vec<String> {
    components
        .iter()
        .map(|component| {
            table
                .get(component)
                .and_then(|item| item.as_str())
                .map(escape)
                .unwrap_or_default()
        })
        .collect()
}

/// Append `;TYPE=<value>` to `line` when the table carries a
/// non-empty `type`.
fn push_type(line: &mut String, table: &dyn TableLike) {
    if let Some(ty) = table
        .get("type")
        .and_then(|item| item.as_str())
        .filter(|ty| !ty.is_empty())
    {
        line.push_str(";TYPE=");
        line.push_str(ty);
    }
}

/// Push a vCard content line with CRLF, as the spec mandates.
fn push_line(out: &mut String, line: &str) {
    out.push_str(line);
    out.push_str("\r\n");
}

/// Collect the TOML tables addressed by an array-of-tables
/// (`[[key]]`) or an inline array of inline tables.
fn tables(item: &Item) -> Vec<&dyn TableLike> {
    if let Some(array) = item.as_array_of_tables() {
        array.iter().map(|table| table as &dyn TableLike).collect()
    } else if let Some(array) = item.as_array() {
        array
            .iter()
            .filter_map(|value| value.as_inline_table())
            .map(|table| table as &dyn TableLike)
            .collect()
    } else {
        Vec::new()
    }
}

/// First value of an entry as text.
fn entry_text(entry: &VCardEntry) -> Option<&str> {
    entry.values.first().and_then(|value| value.as_text())
}

/// All texts of an entry, flattening structured components.
fn entry_texts(entry: &VCardEntry) -> Vec<String> {
    entry.values.iter().flat_map(value_strings).collect()
}

/// Ordered components of a structured entry (`N`, `ADR`).
fn entry_components(entry: &VCardEntry) -> Vec<String> {
    match entry.values.first() {
        Some(VCardValue::Component(parts)) => parts.clone(),
        _ => entry
            .values
            .iter()
            .filter_map(|value| value.as_text().map(str::to_owned))
            .collect(),
    }
}

/// All texts carried by a single value.
fn value_strings(value: &VCardValue) -> Vec<String> {
    match value {
        VCardValue::Component(parts) => parts.clone(),
        other => other.as_text().map(str::to_owned).into_iter().collect(),
    }
}

/// `TYPE` parameter values of an entry, lowercased.
fn type_strings(entry: &VCardEntry) -> Vec<String> {
    entry
        .parameters(&VCardParameterName::Type)
        .filter_map(param_text)
        .collect()
}

/// Text form of a parameter value, for `TYPE`.
fn param_text(value: &VCardParameterValue) -> Option<String> {
    match value {
        VCardParameterValue::Text(text) => Some(text.clone()),
        VCardParameterValue::Type(ty) => Some(ty.as_str().to_lowercase()),
        _ => None,
    }
}

/// True unless the property is a structural marker calcard emits on
/// its own (`BEGIN`, `END`, `VERSION`).
fn is_data(name: &VCardProperty) -> bool {
    !matches!(
        name,
        VCardProperty::Begin | VCardProperty::End | VCardProperty::Version
    )
}

/// True when the property is part of the modeled vocabulary.
fn is_modeled(name: &VCardProperty) -> bool {
    FIELDS.iter().any(|field| field.name == name.as_str())
}

/// Render a string as a quoted, escaped TOML scalar.
fn toml_str(value: &str) -> String {
    toml_edit::Value::from(value).to_string().trim().to_string()
}

/// Render strings as a TOML array.
fn toml_array<S: AsRef<str>>(items: &[S]) -> String {
    let mut array = toml_edit::Array::new();

    for item in items {
        array.push(item.as_ref());
    }

    array.to_string().trim().to_string()
}

/// Escape a vCard text value per RFC 6350 section 3.4.
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());

    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            ',' => out.push_str("\\,"),
            ';' => out.push_str("\\;"),
            '\n' => out.push_str("\\n"),
            _ => out.push(ch),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use calcard::vcard::VCardVersion;

    use crate::vcard;

    const SAMPLE: &str = "BEGIN:VCARD\r\n\
        VERSION:4.0\r\n\
        FN:John Doe\r\n\
        N:Doe;John;;;\r\n\
        EMAIL;TYPE=work:john@work.example\r\n\
        EMAIL;TYPE=home:john@home.example\r\n\
        ADR;TYPE=home:;;123 Main St;Springfield;IL;62701;USA\r\n\
        X-CUSTOM;TYPE=weird:keep me verbatim\r\n\
        END:VCARD\r\n";

    #[test]
    fn project_prefills_known_fields() {
        let card = vcard::parse(SAMPLE).unwrap();
        let toml = super::project(&card, VCardVersion::V4_0);

        assert!(toml.contains("fn = \"John Doe\""));
        assert!(toml.contains("family = \"Doe\""));
        assert!(toml.contains("value = \"john@work.example\""));
        assert!(toml.contains("street = \"123 Main St\""));
        // Unmodeled properties never appear in the scaffold.
        assert!(!toml.contains("X-CUSTOM"));
    }

    #[test]
    fn blank_project_layout() {
        let toml = super::project(&Default::default(), VCardVersion::V4_0);

        // uid leads, fn follows; categories and lang sit below role;
        // note is the last bare key; photo precedes url.
        assert!(toml.find("uid =").unwrap() < toml.find("fn =").unwrap());
        assert!(toml.find("role =").unwrap() < toml.find("categories =").unwrap());
        assert!(toml.find("categories =").unwrap() < toml.find("lang =").unwrap());
        assert!(toml.find("lang =").unwrap() < toml.find("note =").unwrap());
        assert!(toml.find("[[photo]]").unwrap() < toml.find("[[url]]").unwrap());

        // Empty, uncommented fields; note as an empty literal.
        assert!(toml.contains("fn = \"\""));
        assert!(toml.contains("note = ''''''"));
        assert!(!toml.contains("#fn"));

        // FN is flagged required; hints use the e.g. form.
        assert!(toml.contains("# required"));
        assert!(toml.contains("# e.g. geo:37.78,-122.40"));
        assert!(toml.contains("# e.g. home work cell"));
        assert!(toml.contains("# e.g. file:// or http://"));
    }

    #[test]
    fn blank_bare_hints_share_a_column() {
        let toml = super::project(&Default::default(), VCardVersion::V4_0);

        let column = |needle: &str| -> usize {
            let line = toml.lines().find(|line| line.contains(needle)).unwrap();
            line.find('#').unwrap()
        };

        // fn, bday, anniversary, geo, tz all align in the bare block.
        assert_eq!(column("bday ="), column("fn ="));
        assert_eq!(column("bday ="), column("anniversary ="));
        assert_eq!(column("bday ="), column("geo ="));
        assert_eq!(column("bday ="), column("tz ="));
    }

    #[test]
    fn photo_has_no_type_line() {
        let toml = super::project(&Default::default(), VCardVersion::V4_0);
        let photo = toml.split("[[photo]]").nth(1).unwrap();

        assert!(!photo.lines().take(2).any(|line| line.starts_with("type =")));
    }

    #[test]
    fn apply_roundtrip_preserves_unknown_properties() {
        let card = vcard::parse(SAMPLE).unwrap();
        let toml = super::project(&card, VCardVersion::V4_0);

        let out = super::apply(&card, &toml, VCardVersion::V4_0).unwrap();

        assert!(out.contains("FN:John Doe"));
        assert!(out.contains("john@work.example"));
        assert!(out.contains("john@home.example"));
        // The unmodeled property survives the round-trip verbatim.
        assert!(out.contains("X-CUSTOM"));
        assert!(out.contains("keep me verbatim"));
    }

    #[test]
    fn project_then_apply_preserves_bare_fields_after_sections() {
        // These scalar/list fields are emitted before the sections so
        // TOML does not nest them inside a table; a round-trip through
        // the projected scaffold must keep every one of them.
        let filled = "BEGIN:VCARD\r\n\
            VERSION:4.0\r\n\
            FN:Ada Lovelace\r\n\
            NICKNAME:Ada\r\n\
            NOTE:Pioneer\r\n\
            CATEGORIES:science\r\n\
            UID:urn:uuid:1234\r\n\
            EMAIL;TYPE=work:ada@analytical.example\r\n\
            END:VCARD\r\n";
        let card = vcard::parse(filled).unwrap();
        let toml = super::project(&card, VCardVersion::V4_0);

        let out = super::apply(&card, &toml, VCardVersion::V4_0).unwrap();

        assert!(out.contains("NICKNAME:Ada"));
        assert!(out.contains("NOTE:Pioneer"));
        assert!(out.contains("CATEGORIES:science"));
        assert!(out.contains("UID:urn:uuid:1234"));
        assert!(out.contains("ada@analytical.example"));
    }

    #[test]
    fn apply_ignores_empty_fields() {
        let card = vcard::parse(SAMPLE).unwrap();
        // A whole blank form must drop every modeled field (all empty)
        // yet keep the unknown property.
        let blank = super::project(&Default::default(), VCardVersion::V4_0);

        let out = super::apply(&card, &blank, VCardVersion::V4_0).unwrap();

        assert!(!out.contains("FN:"));
        assert!(!out.contains("EMAIL"));
        assert!(out.contains("X-CUSTOM"));
    }

    #[test]
    fn apply_edits_modeled_field() {
        let card = vcard::parse(SAMPLE).unwrap();
        let edited = "fn = \"Jane Roe\"\n";

        let out = super::apply(&card, edited, VCardVersion::V4_0).unwrap();

        assert!(out.contains("FN:Jane Roe"));
        assert!(!out.contains("John Doe"));
        assert!(out.contains("X-CUSTOM"));
    }
}
