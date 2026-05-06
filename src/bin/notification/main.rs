use bevy::{
    color::palettes::basic::*,
    log::{
        Level, LogPlugin,
        tracing_subscriber::{self, Layer},
    },
    prelude::*,
    window::WindowMode,
    winit::WinitSettings,
};
use objc2_foundation::{NSArray, NSMutableSet, NSString, ns_string};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNMutableNotificationContent, UNNotificationAction,
    UNNotificationActionIcon, UNNotificationActionOptions, UNNotificationCategory,
    UNNotificationCategoryOptions, UNUserNotificationCenter,
};

use crate::notification::{
    Notification, NotificationAuthorized, NotificationCenter, NotificationPlugin,
    NotificationPresented, NotificationResponded, NotificationScheduled,
};

mod notification;

fn main() {
    App::new()
        // Boilerplate for making iOS work
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    level: Level::DEBUG,
                    filter: "error,notification=debug".to_string(),
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
        .add_plugins(NotificationPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                button_handler,
                start_after_authorization,
                handle_scheduling,
                handle_presented,
                handle_responded,
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.insert_resource(Position { x: 5, y: 5 });

    commands.spawn((
        Item {
            name: "rusty sword (+1 ATK)".into(),
            symbol: "🟪".into(),
        },
        Position { x: 7, y: 8 },
    ));
    commands.spawn((
        Item {
            name: "worn shoe".into(),
            symbol: "🟨".into(),
        },
        Position { x: 3, y: 4 },
    ));

    let center = UNUserNotificationCenter::currentNotificationCenter();

    let categories = NSMutableSet::new();
    categories.addObject(
        &*UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
            &NSString::from_str("MOVE"),
            &NSArray::from_retained_slice(&[
                UNNotificationAction::actionWithIdentifier_title_options_icon(
                    ns_string!("ACTION_LEFT"),
                    ns_string!("Go left"),
                    UNNotificationActionOptions::empty(),
                    Some(&UNNotificationActionIcon::iconWithSystemImageName(
                        ns_string!("arrowshape.left"),
                    )),
                ),
                UNNotificationAction::actionWithIdentifier_title_options_icon(
                    ns_string!("ACTION_RIGHT"),
                    ns_string!("Go right"),
                    UNNotificationActionOptions::empty(),
                    Some(&UNNotificationActionIcon::iconWithSystemImageName(
                        ns_string!("arrowshape.right"),
                    )),
                ),
                UNNotificationAction::actionWithIdentifier_title_options_icon(
                    ns_string!("ACTION_UP"),
                    ns_string!("Go up"),
                    UNNotificationActionOptions::empty(),
                    Some(&UNNotificationActionIcon::iconWithSystemImageName(
                        ns_string!("arrowshape.up"),
                    )),
                ),
                UNNotificationAction::actionWithIdentifier_title_options_icon(
                    ns_string!("ACTION_DOWN"),
                    ns_string!("Go down"),
                    UNNotificationActionOptions::empty(),
                    Some(&UNNotificationActionIcon::iconWithSystemImageName(
                        ns_string!("arrowshape.down"),
                    )),
                ),
            ]),
            &NSArray::new(),
            UNNotificationCategoryOptions::CustomDismissAction,
        ),
    );
    categories.addObject(
        &*UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
            &NSString::from_str("TAKE"),
            &NSArray::from_retained_slice(&[
                UNNotificationAction::actionWithIdentifier_title_options_icon(
                    ns_string!("ACTION_TAKE"),
                    ns_string!("Take"),
                    UNNotificationActionOptions::empty(),
                    Some(&UNNotificationActionIcon::iconWithSystemImageName(
                        ns_string!("hand.point.up"),
                    )),
                ),
                UNNotificationAction::actionWithIdentifier_title_options_icon(
                    ns_string!("ACTION_LEAVE"),
                    ns_string!("Leave"),
                    // Not really, but it makes the button red
                    UNNotificationActionOptions::Destructive,
                    Some(&UNNotificationActionIcon::iconWithSystemImageName(
                        ns_string!("figure.walk"),
                    )),
                ),
            ]),
            &NSArray::new(),
            UNNotificationCategoryOptions::CustomDismissAction,
        ),
    );

    // Maximum number of categories seems to be 100
    center.setNotificationCategories(&categories);

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
            Text::new("Start"),
            TextFont {
                font_size: 30.0,
                ..default()
            },
            TextColor::BLACK,
            TextLayout::new_with_justify(Justify::Center),
        ));
}

fn button_handler(
    notification_center: Res<NotificationCenter>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                notification_center.request_authorization(
                    UNAuthorizationOptions::Alert
                        | UNAuthorizationOptions::Sound
                        | UNAuthorizationOptions::Badge,
                );

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

fn start_after_authorization(
    mut reader: MessageReader<NotificationAuthorized>,
    mut commands: Commands,
    notifications: Query<Entity, With<Notification>>,
    player_position: Res<Position>,
    items: Query<(Entity, &Item, &Position)>,
) {
    for NotificationAuthorized { result } in reader.read() {
        match result {
            Ok(true) => {
                // Remove all existing notifications
                for entity in notifications {
                    commands.entity(entity).despawn();
                }

                commands.spawn(map_notification(
                    "Look around for treasure!",
                    *player_position,
                    items,
                ));
            }
            Ok(false) => {
                error!("notification authorization denied");
            }
            Err(error) => {
                error!(%error, "failed authorizing notifications");
            }
        }
    }
}

fn handle_scheduling(
    mut reader: MessageReader<NotificationScheduled>,
    notifications: Query<&Notification>,
) {
    for NotificationScheduled {
        notification,
        result,
    } in reader.read()
    {
        let notification = notifications.get(*notification).unwrap();
        match result {
            Ok(()) => {
                debug!(?notification, "presented");
            }
            Err(error) => {
                error!(?notification, %error, "failed scheduling notification");
            }
        }
    }
}

fn handle_presented(
    mut reader: MessageReader<NotificationPresented>,
    notifications: Query<&Notification>,
) {
    for NotificationPresented { notification } in reader.read() {
        let notification = notifications.get(*notification).unwrap();
        debug!(?notification, "presented");
    }
}

fn handle_responded(
    mut commands: Commands,
    mut reader: MessageReader<NotificationResponded>,
    notifications: Query<&Notification>,
    mut player_position: ResMut<Position>,
    items: Query<(Entity, &Item, &Position)>,
) {
    for NotificationResponded {
        notification,
        action_identifier,
    } in reader.read()
    {
        // Despawn the entity after this event is handled.
        commands.entity(*notification).despawn();

        let notification = notifications.get(*notification).unwrap();
        debug!(?notification, action_identifier, "responded");

        if &*notification.content().categoryIdentifier() == ns_string!("MOVE") {
            match &**action_identifier {
                "ACTION_LEFT" => {
                    player_position.x -= 1;
                }
                "ACTION_RIGHT" => {
                    player_position.x += 1;
                }
                "ACTION_UP" => {
                    player_position.y -= 1;
                }
                "ACTION_DOWN" => {
                    player_position.y += 1;
                }
                _ => {}
            }

            if let Some((_, current_item, _)) =
                items.iter().find(|(_, _, pos)| **pos == *player_position)
            {
                let content = UNMutableNotificationContent::new();
                content.setTitle(&NSString::from_str(&format!(
                    "You come across a {}",
                    current_item.name
                )));
                content.setBody(ns_string!("Do you want to pick it up?"));
                content.setCategoryIdentifier(ns_string!("TAKE"));
                commands.spawn(Notification::new(&content));
            } else {
                commands.spawn(map_notification("Walkin'", *player_position, items));
            }
        } else if &*notification.content().categoryIdentifier() == ns_string!("TAKE") {
            match &**action_identifier {
                "ACTION_TAKE" => {
                    let (entity, current_item, _) = items
                        .iter()
                        .find(|(_, _, pos)| **pos == *player_position)
                        .unwrap();
                    commands.entity(entity).despawn();

                    commands.spawn(map_notification(
                        &format!("You grab the {}!", current_item.name),
                        *player_position,
                        items,
                    ));
                }
                "ACTION_LEAVE" => {
                    let (_, current_item, _) = items
                        .iter()
                        .find(|(_, _, pos)| **pos == *player_position)
                        .unwrap();

                    commands.spawn(map_notification(
                        &format!("You leave the {}, and move on", current_item.name),
                        *player_position,
                        items,
                    ));
                }
                _ => {}
            }
        }
    }
}

fn map_notification(
    text: &str,
    player_position: Position,
    items: Query<(Entity, &Item, &Position)>,
) -> Notification {
    let content = UNMutableNotificationContent::new();
    content.setTitle(&NSString::from_str(text));

    let mut visible_map = String::new();
    for y in 0..10 {
        for x in 0..10 {
            let position = Position { x, y };

            let mut symbol = "⬛️";

            for (_, item, pos) in &items {
                if *pos == position {
                    symbol = &item.symbol;
                }
            }

            if position == player_position {
                symbol = "🏃";
            }

            visible_map += symbol;
        }
        visible_map += "\n";
    }

    content.setBody(&NSString::from_str(&visible_map));
    content.setCategoryIdentifier(ns_string!("MOVE"));
    // content.setAttachments(&NSArray::from_slice(&[UNNotificationAttachment]));
    // content.setRelevanceScore(0.0);
    // content.setSound(Some(&UNNotificationSound::todo!()));
    Notification::new(&content)
}

#[derive(Component)]
struct Item {
    name: String,
    symbol: String,
}

#[derive(Component, Resource, Clone, Copy, PartialEq, Eq, Hash)]
struct Position {
    x: usize,
    y: usize,
}
