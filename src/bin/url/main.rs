use bevy::{
    color::palettes::css::*,
    ecs::system::NonSendMarker,
    log::{
        Level, LogPlugin,
        tracing_subscriber::{self, Layer},
    },
    prelude::*,
    window::WindowMode,
    winit::WinitSettings,
};
use block2::RcBlock;
use objc2::{MainThreadMarker, runtime::Bool};
use objc2_foundation::{NSDictionary, NSURL, ns_string};
use objc2_ui_kit::UIApplication;

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
        .add_systems(Update, button_handler)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn((
            Button,
            Node {
                justify_self: JustifySelf::Center,
                align_self: AlignSelf::Center,
                ..default()
            },
        ))
        .with_child((
            Text::new("Open URL"),
            TextFont {
                font_size: 30.0,
                ..default()
            },
            TextColor::BLACK,
            TextLayout::new_with_justify(Justify::Center),
        ));
}

fn button_handler(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    _: NonSendMarker,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                let mtm = MainThreadMarker::new().unwrap();
                let app = UIApplication::sharedApplication(mtm);

                unsafe {
                    app.openURL_options_completionHandler(
                        &NSURL::URLWithString(ns_string!("https://example.com")).unwrap(),
                        &NSDictionary::new(),
                        Some(&RcBlock::new(|success: Bool| {
                            if !success.as_bool() {
                                error!("Failed to open URL");
                            } else {
                                info!("successfully opened URL");
                            }
                        })),
                    )
                };

                *color = BLUE.into();
            }
            Interaction::Hovered => {
                *color = GRAY.into();
            }
            Interaction::None => {
                *color = WHITE.into();
            }
        }
    }
}
