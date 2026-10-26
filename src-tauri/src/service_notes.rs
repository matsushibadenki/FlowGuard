use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub label: String,
    pub comment: String,
    pub revision: u64,
    pub updated_at: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Edit {
    pub port: u16,
    pub expected_revision: u64,
    pub label: String,
    pub comment: String,
}
pub fn edit(notes: &mut BTreeMap<u16, Note>, command: Edit, now: u64) -> Result<(), String> {
    if command.label.chars().count() > 80
        || command.comment.chars().count() > 500
        || command.label.chars().any(char::is_control)
        || command
            .comment
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err("invalid_service_note".into());
    }
    let previous = notes.get(&command.port).cloned().unwrap_or_default();
    if previous.revision != command.expected_revision {
        return Err("service_note_conflict".into());
    }
    let revision = previous
        .revision
        .checked_add(1)
        .ok_or("service_note_revision_exhausted")?;
    // Keep an empty note's revision so stale windows cannot recreate deleted text.
    notes.insert(
        command.port,
        Note {
            label: command.label,
            comment: command.comment,
            revision,
            updated_at: now,
        },
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn command(expected_revision: u64, label: &str, comment: &str) -> Edit {
        Edit {
            port: 443,
            expected_revision,
            label: label.into(),
            comment: comment.into(),
        }
    }
    #[test]
    fn unicode_bounds_and_controls_are_checked_without_mutation() {
        let mut notes = BTreeMap::new();
        edit(
            &mut notes,
            command(0, &"界".repeat(80), &"😀".repeat(500)),
            1,
        )
        .unwrap();
        let original = notes.clone();
        for (label, comment) in [
            ("界".repeat(81).as_str(), "ok"),
            ("ok", "😀".repeat(501).as_str()),
            ("a\nb", "ok"),
            ("a", "bad\0"),
            ("a", "bad\u{1b}"),
        ] {
            assert!(edit(&mut notes, command(1, label, comment), 2).is_err());
            assert_eq!(notes, original);
        }
        edit(&mut notes, command(1, "<display>", "line1\nline2\ttab"), 2).unwrap();
    }
    #[test]
    fn old_window_edits_are_rejected_even_after_text_is_cleared() {
        let mut notes = BTreeMap::new();
        edit(&mut notes, command(0, "web", "check"), 1).unwrap();
        edit(&mut notes, command(1, "", ""), 2).unwrap();
        assert_eq!(
            edit(&mut notes, command(0, "old", "old"), 3).unwrap_err(),
            "service_note_conflict"
        );
        assert_eq!(notes[&443].revision, 2);
        assert!(notes[&443].label.is_empty());
    }
}
