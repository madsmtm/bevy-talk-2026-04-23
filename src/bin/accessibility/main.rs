mod accessibility_bold_text;
mod preferred_content_size_category;
use bevy::{
    log::{
        Level, LogPlugin,
        tracing_subscriber::{self, Layer},
    },
    prelude::*,
    window::WindowMode,
    winit::WinitSettings,
};

use crate::preferred_content_size_category::{
    PreferredContentSizeCategory, PreferredContentSizeCategoryPlugin,
};

#[cfg(target_os = "ios")]
embed_plist::embed_info_plist!("Info.plist");

fn main() {
    App::new()
        // Boilerplate for making iOS work
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    level: Level::DEBUG,
                    filter: "error,accessibility=debug".to_string(),
                    custom_layer: |_| {
                        Some(
                            tracing_subscriber::fmt::Layer::default()
                                .with_writer(std::io::stderr)
                                .boxed(),
                        )
                    },
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(WinitSettings::mobile())
        // Actual app
        .add_systems(Startup, setup)
        // Helper for content
        .add_plugins(PreferredContentSizeCategoryPlugin::new())
        .add_systems(PreUpdate, scale_auto_text)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Text2d::new("Hello World!"),
        TextLayout::new_with_justify(Justify::Center),
        AutoText { font_size: 20.0 },
    ));
}

#[derive(Component)]
#[require(TextFont)]
struct AutoText {
    font_size: f32,
}

fn scale_auto_text(
    preferred_content_size: Res<PreferredContentSizeCategory>,
    fonts: Query<(&AutoText, &mut TextFont)>,
) {
    if preferred_content_size.is_changed() {
        info!("changed to: {preferred_content_size:?}");
    }
    for (auto_text, mut font) in fonts {
        // Semi-random values that make the font bigger/smaller depending on
        // preferred content size category. Real-world usage should probably
        // use something more sophisticated.
        let new = match *preferred_content_size {
            PreferredContentSizeCategory::ExtraSmall => 0.6,
            PreferredContentSizeCategory::Small => 0.8,
            PreferredContentSizeCategory::Medium => 1.0,
            PreferredContentSizeCategory::Large => 1.2,
            PreferredContentSizeCategory::ExtraLarge => 1.4,
            PreferredContentSizeCategory::ExtraExtraLarge => 1.7,
            PreferredContentSizeCategory::ExtraExtraExtraLarge => 2.0,
            PreferredContentSizeCategory::AccessibilityMedium => 3.0,
            PreferredContentSizeCategory::AccessibilityLarge => 4.0,
            PreferredContentSizeCategory::AccessibilityExtraLarge => 5.0,
            PreferredContentSizeCategory::AccessibilityExtraExtraLarge => 6.0,
            PreferredContentSizeCategory::AccessibilityExtraExtraExtraLarge => 7.0,
            _ => 1.0,
        } * auto_text.font_size;
        if font.font_size != new {
            font.font_size = new;
        }
    }
}
