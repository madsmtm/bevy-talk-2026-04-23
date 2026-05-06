#![allow(dead_code)]
use bevy::{
    app::{App, Plugin, PreUpdate},
    ecs::change_detection::ResMut,
    ecs::resource::Resource,
};
use objc2_ui_kit::UIAccessibilityIsBoldTextEnabled;

/// Whether bold text is desired on labels and such.
///
/// Controlled in `Settings > Accessibility > Display & Text Size > Bold Text`.
#[derive(Resource, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AccessibilityBoldText(pub bool);

pub struct AccessibilityBoldTextPlugin {
    _priv: (),
}

impl AccessibilityBoldTextPlugin {
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

impl Plugin for AccessibilityBoldTextPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AccessibilityBoldText(UIAccessibilityIsBoldTextEnabled()));

        // We _could_ register an observer on
        // `UIAccessibilityBoldTextStatusDidChangeNotification`, but
        // `UIAccessibilityIsBoldTextEnabled()` is fast, so we'll just call
        // that on every update.
        app.add_systems(PreUpdate, |mut bold: ResMut<AccessibilityBoldText>| {
            let new = UIAccessibilityIsBoldTextEnabled();
            if bold.0 != new {
                bold.0 = new;
            }
        });
    }
}
