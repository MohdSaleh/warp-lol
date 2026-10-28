use std::fmt::Display;

use warpui::ui_components::button::ButtonVariant;
use warpui::Action;

/// The Tabs trait provides common functionality for an enum to be used as a tabs menu UI component.
pub trait Tabs: PartialEq + Display + Copy {
    type TabAction: Action + Clone;

    #[allow(dead_code)]
    fn button_variant(&self, selected_view_option: &Self) -> ButtonVariant {
        if self == selected_view_option {
            ButtonVariant::Basic
        } else {
            ButtonVariant::Outlined
        }
    }

    fn tab_name(&self) -> String {
        self.to_string()
    }

    #[allow(dead_code)]
    fn action_on_click(&self, selection: Self) -> Self::TabAction;
}
