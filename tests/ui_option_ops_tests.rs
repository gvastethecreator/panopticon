//! Integration tests for pure UI option helpers.

use panopticon::settings::{AppSelectionEntry, HiddenAppEntry};
use panopticon::ui_option_ops::{
    app_option_label, current_profile_label, hidden_app_option_label, parse_option_value,
    suggested_tag_name, tag_color_hex, tag_color_index, OPTION_SEPARATOR,
};

#[test]
fn current_profile_label_falls_back_to_default() {
    assert_eq!(current_profile_label(None), "default");
    assert_eq!(current_profile_label(Some("workbench")), "workbench");
}

#[test]
fn option_labels_embed_separator_and_roundtrip_app_id() {
    let app = AppSelectionEntry {
        app_id: "app:code".to_owned(),
        label: "Code".to_owned(),
    };
    let hidden = HiddenAppEntry {
        app_id: "app:arc".to_owned(),
        label: "Arc".to_owned(),
    };

    let app_label = app_option_label(&app);
    let hidden_label = hidden_app_option_label(&hidden);

    assert_eq!(app_label, format!("Code{OPTION_SEPARATOR}app:code"));
    assert_eq!(hidden_label, format!("Arc{OPTION_SEPARATOR}app:arc"));
    assert_eq!(parse_option_value(&app_label), Some("app:code".to_owned()));
    assert_eq!(
        parse_option_value(&hidden_label),
        Some("app:arc".to_owned())
    );
}

#[test]
fn parse_option_value_rejects_missing_or_empty_suffix() {
    assert_eq!(parse_option_value("Code"), None);
    assert_eq!(
        parse_option_value(&format!("Code{OPTION_SEPARATOR}   ")),
        None
    );
}

#[test]
fn suggested_tag_name_normalizes_mixed_labels() {
    assert_eq!(
        suggested_tag_name("Visual Studio Code"),
        "visual studio code"
    );
    assert_eq!(suggested_tag_name("OBS-Studio!!!"), "obs studio");
    assert_eq!(suggested_tag_name("  ---  "), "");
}

#[test]
fn tag_color_mappings_roundtrip_all_presets() {
    let expected = [
        (0, "D29A5C"),
        (1, "5CA9FF"),
        (2, "3CCF91"),
        (3, "FF6B8A"),
        (4, "9B7BFF"),
        (5, "F4B740"),
    ];

    for (index, hex) in expected {
        assert_eq!(tag_color_hex(index), hex);
        assert_eq!(tag_color_index(hex), index);
    }
}

#[test]
fn unknown_tag_color_defaults_to_amber_preset() {
    assert_eq!(tag_color_index("abcdef"), 0);
    assert_eq!(tag_color_hex(99), "D29A5C");
}
