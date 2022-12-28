/// Functions for rendering the help window
use std::collections::HashMap;
use crossterm::{
    style::StyledContent,
    event::KeyEvent,
};
use crate::ui::{
    markup_render::{wrap_and_stylize, README_STR},
    Action, ActionContext,
};


/// Word-wrap the help string to be displayed in the help window, and apply correct formatting
/// (such as bolding) using crossterm::style.
///
/// Returns a vector of vectors, where the outer vector represents lines, and the inner vector
/// contains either a single string for the whole line, or multiple strings, if the style varies
/// within the line.
pub fn get_formatted_help_text(
    width: usize,
    key_mapping: &HashMap<(KeyEvent, ActionContext), Action>,
) -> Vec<Vec<StyledContent<String>>> {
    let help_str = &README_STR[
        README_STR.find("## User guide").expect("Could not find user guide in README")
        ..
        README_STR.find("## Similar projects").expect("Could not find end of user guide in README")
    ];

    // Skip the table of keyboard shortcuts, we'll format it separately
    let (help_str, rest) = help_str
        .split_once("\n\n|")
        .expect("Could not find keyboard shortcuts table in readme");

    let rest = rest
        .split_once("\n\n")
        .expect("Could not find end of keyboard shortcuts table in readme")
        .1;

    // Add justified keyboard shortcuts table to help string
    let mut help_str = help_str.to_string();
    help_str.push_str("\n\n"); // add back newlines eaten by split_once
    help_str = help_str.replace("shortcuts by default", "shortcuts"); // we're going to display the actual shortcuts
    help_str.push_str(&get_justified_keyboard_shortcuts_table(key_mapping));

    let rest = rest
        // Remove mention of 'vim-like' shortcuts, it might not apply if the user has customized them.
        .split('\n').filter(|line| !line.contains("should be familiar to")).collect::<Vec<_>>().join("\n");
    help_str.push_str(&rest);

    // We need to get rid of the `<kbd>` tags before wrapping so it works correctly. We're going to
    // bold all words within backticks, so replace the tags with backticks as well.
    let help_str = help_str
        .replace("<kbd>",  "`")
        .replace("</kbd>", "`");

    wrap_and_stylize(&help_str, width)
}


/// Extract the table of keyboard shortcuts from the README. Panics if the README is incorrectly
/// formatted.
fn get_keyboard_shortcuts_table() -> &'static str {
    let keyboard_shortcuts = README_STR
        .split_once("keyboard shortcuts by default:\n\n")
        .expect("Couldn't find table of keyboard shortcuts in README")
        .1;
    let keyboard_shortcuts = keyboard_shortcuts
        .split_once("\n\n")
        .expect("Couldn't find end of keyboard shortcuts table in README")
        .0;

    keyboard_shortcuts
}

/// Apply justification to the table of keyboard shortcuts in the README and render it to a String
/// with the markdown table markup and <kbd> tags removed.
fn get_justified_keyboard_shortcuts_table(
    key_mapping: &HashMap<(KeyEvent, ActionContext), Action>,
) -> String {
    let formatter = crokey::KeyEventFormat::default();

    let keyboard_shortcuts = get_keyboard_shortcuts_table();

    let first_column_width = keyboard_shortcuts
        .lines()
        .map(|line| line.split('|').nth(1).unwrap_or("").replace('`', "").len())
        .max()
        .unwrap_or(10);

    // Mapping from action names to list of key combinations
    let key_mapping_inv = invert_key_mapping_sorted(key_mapping);

    let mut justified = String::new();

    for (i, line) in keyboard_shortcuts.lines().enumerate() {
        // cols[0] is empty, because the lines start with '|'.
        let cols: Vec<&str> = line.split('|').map(|c| c.trim()).collect();

        let (action_desc, shortcuts) = match i {
            0 => {
                // first row is the headers, add backticks to bold them
                (
                    format!("`{}`", cols[1]),
                    format!("`{}`", cols[2].replace("Default s", "S")),
                )
            }
            1 => continue, // skip row containing markdown table formatting
            _ => {
                let action_name = cols[3].replace('`', "").trim().to_string();
                // add backticks + short description of context to each key combo
                let shortcuts_formatted: String = match key_mapping_inv.get(&action_name) {
                    Some(shortcuts) => shortcuts
                        .iter()
                        .map(|(keys, ctx)| {
                            let mut shortcut = format!("`{}`", formatter.to_string(*keys));
                            let ctx = match ctx {
                                ActionContext::None => String::new(),
                                _ => format!(" ({})", ctx.short_description()),
                            };
                            shortcut.push_str(&ctx);
                            shortcut
                        })
                        .collect::<Vec<String>>()
                        .join(", "),
                    None => "No mapping found".to_string(),
                };

                (cols[1].to_string(), shortcuts_formatted)
            }
        };

        justified.push_str(&action_desc);

        // backticks will be removed, so add extra space for them
        let extra_len = action_desc.chars().filter(|c| *c == '`').count();
        let padding = first_column_width + extra_len + 2 - action_desc.len();
        justified.push_str(&" ".repeat(padding));
        justified.push_str(&shortcuts);
        // It's ok to add "\n" at the end of every line, because the split_once() above has
        // eaten too many newlines from the end anyway.
        justified.push('\n');
    }

    justified
}

/// Invert a key mapping, so that we have a mapping from action names to list of key combinations.
/// Each list of key combinations is sorted for display purposes, that is, keys with no modifiers
/// are placed first (alphabetically), and keys with contexts other than None are placed last.
fn invert_key_mapping_sorted(
    key_mapping: &HashMap<(KeyEvent, ActionContext), Action>,
) -> HashMap<String, Vec<(KeyEvent, ActionContext)>> {
    let mut key_mapping_inv = HashMap::new();

    // compare two key events: put keys without modifiers before those that have modifiers this is
    // probably not the right place for this, but I'll move
    // it out if I need it elsewhere.
    fn cmp_key_events(k1: &KeyEvent, k2: &KeyEvent) -> std::cmp::Ordering {
        let formatter = crokey::KeyEventFormat::default();
        match (k1.modifiers.is_empty(), k2.modifiers.is_empty()) {
            (true, true) | (false, false) => {
                // both or neither have modifiers, sort alphabetically
                formatter.to_string(*k1).cmp(&formatter.to_string(*k2))
            }
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
        }
    }

    // Collect all mappings corresponding to a given action
    for ((k, c), a) in key_mapping {
        key_mapping_inv
            .entry(a.to_string())
            .or_insert(vec![])
            .push((*k, c.clone()))
    }

    // Sort the key mappings, they are in a random order because of hashmap
    for (_, mappings) in key_mapping_inv.iter_mut() {
        mappings.sort_unstable_by(|(k1, c1), (k2, c2)| match (c1, c2) {
            (ActionContext::None, ActionContext::None) => cmp_key_events(k1, k2),
            (_,                   ActionContext::None) => std::cmp::Ordering::Greater,
            (ActionContext::None,                   _) => std::cmp::Ordering::Less,
            (_, _) => c1.to_string().cmp(&c2.to_string()),
        })
    }

    key_mapping_inv
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_guide_found() {
        // this should panic if the README is incorrectly formatted
        get_formatted_help_text(100, &HashMap::new());
    }

    #[test]
    fn test_replace_works_as_expected() {
        // The get_formatted_help_text function does a bunch of replacements. Here we test that all
        // of the things that are being replaced are found in the README.
        assert!(README_STR.contains("shortcuts by default"));
        assert!(README_STR.contains("<kbd>"));
        assert!(README_STR.contains("</kbd>"));
        assert!(README_STR.contains("Default s"));
        assert!(README_STR.contains("should be familiar to"));
    }

    #[test]
    fn test_keyboard_shortcuts_table_fond() {
        let table = get_keyboard_shortcuts_table();
        let lines: Vec<_> = table.split('\n').collect();
        assert_eq!(lines[0].chars().next().unwrap(), '|');
        assert_eq!(lines[0].chars().last().unwrap(), '|');

        assert_eq!(lines.iter().last().unwrap().chars().next().unwrap(), '|');
        assert_eq!(lines.iter().last().unwrap().chars().last().unwrap(), '|');

        assert!(lines[0].contains("Description"));
    }

    #[test]
    fn test_all_key_mappings_listed_in_readme() {
        use std::str::FromStr;
        use strum::IntoEnumIterator;

        let table_lines: Vec<_> = get_keyboard_shortcuts_table().split('\n').skip(2).collect();

        let mut key_mappings: HashMap<KeyEvent, Vec<Action>> = HashMap::new();

        table_lines.iter().for_each(|line| {
            let parts: Vec<_> = line.split('|').collect();

            let action_name = parts[3].replace('`', "").trim().to_string();
            let action = Action::from_str(&action_name).unwrap_or_else(|_| {
                panic!("Invalid action in table row '{}': '{}'", line, action_name)
            });

            let key_combos: Vec<_> = parts[2]
                .replace("if not searching,", "").replace("if searching", "")
                .replace("<kbd>", "").replace("</kbd>", "")
                .replace('+', "-")
                .replace('↑', "up").replace('↓', "down").replace('←', "left").replace('→', "right")
                .replace("Page Up", "pageup").replace("Page Down", "pagedown")
                .split(" or ")
                .map(|k| crokey::parse(k.trim()).unwrap())
                .collect();
            for k in key_combos {
                key_mappings
                    .entry(k)
                    .and_modify(|a| a.push(action.clone()))
                    .or_insert_with(|| vec![action.clone()]);
            }
        });

        // Check that all actions are listed
        let actions: Vec<_> = key_mappings.values().flatten().collect();
        for action in Action::iter() {
            if action != Action::None {
                assert!(
                    actions.contains(&&action),
                    "Action '{}' not found in readme",
                    action
                );
            }
        }

        // Check that default keymaps match the ones listed in the README
        for (key_combo, _, expected_action) in crate::settings::DEFAULT_KEYMAP {
            // Shift-'?' doesn't need to be in the readme, it's in the default keymap to fix
            // the behavior on Windows.
            if *key_combo == crokey::key!(shift-'?') {
                continue;
            }

            let key_combo_str = crokey::KeyEventFormat::default().to_string(*key_combo);
            let actions = key_mappings.get(key_combo).unwrap_or_else(|| {
                panic!(
                    "Key mapping {}:{} not found in README",
                    key_combo_str, expected_action,
                )
            });
            assert!(
                actions.contains(expected_action),
                "Key mapping '{}:{}' in default keymap doesn't match README: '{:?}'",
                key_combo_str,
                expected_action,
                actions
            );
        }
    }
}
