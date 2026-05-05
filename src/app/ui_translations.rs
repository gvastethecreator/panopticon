//! Slint translation/global binding extracted from `main.rs`.

use slint::{ComponentHandle, SharedString};

pub(crate) fn populate_tr_global<Component>(window: &Component)
where
    Component: ComponentHandle,
    for<'a> crate::Tr<'a>: slint::Global<'a, Component>,
{
    let tr = window.global::<crate::Tr>();
    populate_common_tr(&tr);
    crate::app::settings::translations::populate_settings_tr(&tr);
    populate_tag_tr(&tr);
}

fn populate_common_tr(tr: &crate::Tr<'_>) {
    use panopticon::i18n;

    macro_rules! set_tr {
        ($tr:ident, $setter:ident, $key:literal) => {
            $tr.$setter(SharedString::from(i18n::t($key)));
        };
    }

    set_tr!(tr, set_app_name, "app.name");
    set_tr!(tr, set_main_window_title, "window.main_title");
    set_tr!(tr, set_settings_window_title, "window.settings_title");
    set_tr!(tr, set_tag_window_title, "window.tag_title");
    set_tr!(tr, set_about_window_title, "window.about_title");
    tr.set_minimized(SharedString::from(i18n::t("ui.minimized")));
    tr.set_last_seen(SharedString::from(i18n::t("ui.last_seen")));
    tr.set_visible_label(SharedString::from(i18n::t("ui.visible")));
    tr.set_hidden_label(SharedString::from(i18n::t("ui.hidden")));
    tr.set_always_on_top_label(SharedString::from(i18n::t("ui.always_on_top")));
    tr.set_normal_window_label(SharedString::from(i18n::t("ui.normal_window")));
    tr.set_toolbar_hint(SharedString::from(i18n::t("ui.toolbar_hint")));
    tr.set_anim_on(SharedString::from(i18n::t("ui.anim_on")));
    tr.set_anim_off(SharedString::from(i18n::t("ui.anim_off")));
    set_tr!(tr, set_layout_grid_label, "layout.grid");
    set_tr!(tr, set_layout_mosaic_label, "layout.mosaic");
    set_tr!(tr, set_layout_bento_label, "layout.bento");
    set_tr!(tr, set_layout_fibonacci_label, "layout.fibonacci");
    set_tr!(tr, set_layout_columns_label, "layout.columns");
    set_tr!(tr, set_layout_row_label, "layout.row");
    set_tr!(tr, set_layout_column_label, "layout.column");
    set_tr!(tr, set_group_none_label, "group.none");
    set_tr!(tr, set_group_application_label, "group.application");
    set_tr!(tr, set_group_monitor_label, "group.monitor");
    set_tr!(tr, set_group_title_label, "group.title");
    set_tr!(tr, set_group_class_label, "group.class");
    set_tr!(tr, set_dock_none_label, "dock.none");
    set_tr!(tr, set_dock_left_label, "dock.left");
    set_tr!(tr, set_dock_right_label, "dock.right");
    set_tr!(tr, set_dock_top_label, "dock.top");
    set_tr!(tr, set_dock_bottom_label, "dock.bottom");
    set_tr!(tr, set_locale_english_label, "locale.english");
    set_tr!(tr, set_locale_spanish_label, "locale.spanish");
    set_tr!(tr, set_fit_cover_label, "settings.fit.cover");
    set_tr!(tr, set_fit_contain_label, "settings.fit.contain");
    set_tr!(tr, set_fit_fill_label, "settings.fit.fill");
    set_tr!(tr, set_fit_preserve_label, "settings.fit.preserve");
    set_tr!(tr, set_all_monitors_label, "tray.all_monitors");
    set_tr!(tr, set_all_tags_label, "tray.all_tags");
    set_tr!(tr, set_all_apps_label, "tray.all_apps");
    set_tr!(tr, set_no_hidden_label, "settings.no_hidden");
    set_tr!(
        tr,
        set_no_saved_profiles_label,
        "settings.no_saved_profiles"
    );
    set_tr!(tr, set_default_profile_label, "settings.default_profile");
    tr.set_empty_message(SharedString::from(i18n::t("ui.empty_message")));
    tr.set_empty_helper(SharedString::from(i18n::t("ui.empty_helper")));
    set_tr!(tr, set_action_close, "action.close");
    set_tr!(tr, set_action_reset_defaults, "action.reset_defaults");
    set_tr!(tr, set_action_restore_selected, "action.restore_selected");
    set_tr!(tr, set_action_restore_all, "action.restore_all");
    set_tr!(tr, set_action_browse_image, "action.browse_image");
    set_tr!(tr, set_action_clear_image, "action.clear_image");
    set_tr!(tr, set_action_refresh_now, "action.refresh_now");
    set_tr!(tr, set_action_check_updates, "action.check_updates");
    set_tr!(tr, set_action_auto_apply, "action.auto_apply");
    set_tr!(tr, set_action_about, "action.about");
    set_tr!(tr, set_action_load_profile, "action.load_profile");
    set_tr!(tr, set_about_title, "about.title");
    set_tr!(tr, set_about_subtitle, "about.subtitle");
    set_tr!(tr, set_about_version_title, "about.version_title");
    set_tr!(tr, set_about_update_available, "about.update_available");
    set_tr!(tr, set_about_description_title, "about.description_title");
    set_tr!(tr, set_about_description_body, "about.description_body");
    set_tr!(tr, set_about_credits_title, "about.credits_title");
    set_tr!(tr, set_about_credits_body, "about.credits_body");
}

fn populate_tag_tr(tr: &crate::Tr<'_>) {
    use panopticon::i18n;

    macro_rules! set_tr {
        ($tr:ident, $setter:ident, $key:literal) => {
            $tr.$setter(SharedString::from(i18n::t($key)));
        };
    }

    tr.set_tag_title(SharedString::from(i18n::t("tag.title")));
    tr.set_tag_app_label(SharedString::from(i18n::t("tag.application")));
    tr.set_tag_name_label(SharedString::from(i18n::t("tag.name_label")));
    tr.set_tag_preset_colour(SharedString::from(i18n::t("tag.preset_colour")));
    tr.set_tag_create_assign(SharedString::from(i18n::t("tag.create_assign")));
    set_tr!(tr, set_tag_colour_amber_label, "tag.color.amber");
    set_tr!(tr, set_tag_colour_sky_label, "tag.color.sky");
    set_tr!(tr, set_tag_colour_mint_label, "tag.color.mint");
    set_tr!(tr, set_tag_colour_rose_label, "tag.color.rose");
    set_tr!(tr, set_tag_colour_violet_label, "tag.color.violet");
    set_tr!(tr, set_tag_colour_sun_label, "tag.color.sun");
}
