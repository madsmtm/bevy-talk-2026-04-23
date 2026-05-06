use std::sync::mpsc;

use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    platform::cell::SyncCell,
    prelude::*,
};
use block2::{Block, RcBlock};
use objc2::{
    AllocAnyThread, DefinedClass, Message as _, define_class, msg_send,
    rc::Retained,
    runtime::{Bool, ProtocolObject},
};
use objc2_foundation::{NSArray, NSCopying, NSError, NSObject, NSObjectProtocol, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNNotification, UNNotificationContent,
    UNNotificationPresentationOptions, UNNotificationRequest, UNNotificationResponse,
    UNUserNotificationCenter, UNUserNotificationCenterDelegate,
};

#[derive(Component, PartialEq, Eq, Debug)]
#[component(on_add, on_despawn)]
pub struct Notification {
    content: Retained<UNNotificationContent>,
}

// Safe because we control all access to `.content`, and therefore know it
// isn't a `UNMutableNotificationContent` (which wouldn't be thread-safe to
// modify from different threads).
unsafe impl Send for Notification {}
unsafe impl Sync for Notification {}

impl Notification {
    // In a real-world app, we'd expose a `NotificationBuilder` instead.
    pub fn new(content: &UNNotificationContent) -> Self {
        Self {
            content: content.copy(),
        }
    }

    pub fn content(&self) -> &UNNotificationContent {
        &self.content
    }

    fn on_add(mut world: DeferredWorld, context: HookContext) {
        let notification = context.entity;
        let identifier = NSString::from_str(&notification.to_bits().to_string());

        let sender = world.resource::<NotificationCenter>().clone();

        let component = world.get_mut::<Self>(notification).unwrap();
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &identifier,
            &component.content,
            None, // Trigger immediately
        );

        let center = UNUserNotificationCenter::currentNotificationCenter();
        center.addNotificationRequest_withCompletionHandler(
            &request,
            Some(&RcBlock::new(move |error: *mut NSError| {
                let error = unsafe { error.as_ref() };
                if let Some(error) = error {
                    sender.send(Event::Scheduled(NotificationScheduled {
                        notification,
                        result: Err(error.retain()),
                    }));
                    return;
                }

                sender.send(Event::Scheduled(NotificationScheduled {
                    notification,
                    result: Ok(()),
                }));
            })),
        );
    }

    fn on_despawn(_world: DeferredWorld, context: HookContext) {
        let identifier = NSString::from_str(&context.entity.to_bits().to_string());

        let center = UNUserNotificationCenter::currentNotificationCenter();
        center.removePendingNotificationRequestsWithIdentifiers(&NSArray::from_slice(&[
            &*identifier,
        ]));
        center.removeDeliveredNotificationsWithIdentifiers(&NSArray::from_slice(&[&*identifier]));
    }
}

#[derive(Message, PartialEq, Eq, Hash, Debug, Clone)]
pub struct NotificationAuthorized {
    pub result: Result<bool, Retained<NSError>>,
}

#[derive(Message, PartialEq, Eq, Hash, Debug, Clone)]
pub struct NotificationScheduled {
    pub notification: Entity,
    pub result: Result<(), Retained<NSError>>,
}

#[derive(Message, PartialEq, Eq, Hash, Debug, Clone)]
pub struct NotificationPresented {
    pub notification: Entity,
}

#[derive(Message, PartialEq, Eq, Hash, Debug, Clone)]
pub struct NotificationResponded {
    pub notification: Entity,
    pub action_identifier: String,
}

pub struct NotificationPlugin;

impl Plugin for NotificationPlugin {
    fn build(&self, app: &mut App) {
        let (sender, receiver) = mpsc::channel();
        let delegate = NotificationDelegate::new(NotificationCenter(sender.clone()));
        app.insert_resource(NotificationCenter(sender.clone()));

        let center = UNUserNotificationCenter::currentNotificationCenter();
        center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

        // `UNUserNotificationCenter` only weakly references the delegate, so
        // we keep a strong reference by storing it in a resource.
        app.insert_non_send_resource(delegate);

        // Move messages.
        let mut receiver = SyncCell::new(receiver);
        app.add_systems(
            PreUpdate,
            move |mut writer_authorized: MessageWriter<NotificationAuthorized>,
                  mut writer_scheduled: MessageWriter<NotificationScheduled>,
                  mut writer_presented: MessageWriter<NotificationPresented>,
                  mut writer_responded: MessageWriter<NotificationResponded>| {
                for event in receiver.get().try_iter() {
                    match event {
                        Event::Authorized(message) => {
                            writer_authorized.write(message);
                        }
                        Event::Scheduled(message) => {
                            writer_scheduled.write(message);
                        }
                        Event::Presented(message) => {
                            writer_presented.write(message);
                        }
                        Event::Responded(message) => {
                            writer_responded.write(message);
                        }
                    }
                }
            },
        );

        app.add_message::<NotificationAuthorized>();
        app.add_message::<NotificationScheduled>();
        app.add_message::<NotificationPresented>();
        app.add_message::<NotificationResponded>();

        // #[derive(Resource)]
        // struct RefreshTimer(Timer);
        //
        // // Keep notifications alive (we don't get an event if they're dismissed).
        // app.add_systems(
        //     PostUpdate,
        //     move |timer: Res<RefreshTimer>, notifications: Query<&Notification>| {
        //         if timer.0.just_finished()
        //         let center = UNUserNotificationCenter::currentNotificationCenter();
        //         center.getPendingNotificationRequestsWithCompletionHandler(&RcBlock::new(
        //             move |pending: NonNull<NSArray<UNNotificationRequest>>| {
        //                 let pending = unsafe { pending.as_ref() };
        //                 let center = UNUserNotificationCenter::currentNotificationCenter();
        //                 center.getDeliveredNotificationsWithCompletionHandler(&RcBlock::new(
        //                     move |delivered: NonNull<NSArray<UNNotification>>| {
        //                         let delivered = unsafe { delivered.as_ref() };
        //                     },
        //                 ));
        //             },
        //         ));
        //     },
        // );
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = NotificationCenter]
    struct NotificationDelegate;

    unsafe impl NSObjectProtocol for NotificationDelegate {}

    #[allow(non_snake_case)]
    unsafe impl UNUserNotificationCenterDelegate for NotificationDelegate {
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn userNotificationCenter_willPresentNotification_withCompletionHandler(
            &self,
            center: &UNUserNotificationCenter,
            notification: &UNNotification,
            completion_handler: &Block<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            trace!(?notification, "willPresentNotification");

            let notification = Entity::from_bits(
                notification
                    .request()
                    .identifier()
                    .to_string()
                    .parse::<u64>()
                    .unwrap(),
            );

            self.ivars()
                .send(Event::Presented(NotificationPresented { notification }));

            completion_handler
                .call((UNNotificationPresentationOptions::List
                    | UNNotificationPresentationOptions::Banner,));
        }

        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn userNotificationCenter_didReceiveNotificationResponse_withCompletionHandler(
            &self,
            center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion_handler: &Block<dyn Fn()>,
        ) {
            trace!(?response, "didReceiveNotificationResponse");

            let notification = Entity::from_bits(
                response
                    .notification()
                    .request()
                    .identifier()
                    .to_string()
                    .parse::<u64>()
                    .unwrap(),
            );

            self.ivars().send(Event::Responded(NotificationResponded {
                notification,
                action_identifier: response.actionIdentifier().to_string(),
            }));

            completion_handler.call(());
        }
    }
);

impl NotificationDelegate {
    fn new(sender: NotificationCenter) -> Retained<Self> {
        let this = Self::alloc().set_ivars(sender);
        unsafe { msg_send![super(this), init] }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Event {
    Authorized(NotificationAuthorized),
    Scheduled(NotificationScheduled),
    Presented(NotificationPresented),
    Responded(NotificationResponded),
}

#[derive(Resource, Debug, Clone)]
pub struct NotificationCenter(mpsc::Sender<Event>);

impl NotificationCenter {
    pub fn request_authorization(&self, options: UNAuthorizationOptions) {
        let sender = self.clone();
        let center = UNUserNotificationCenter::currentNotificationCenter();
        center.requestAuthorizationWithOptions_completionHandler(
            options,
            &RcBlock::new(move |granted: Bool, error: *mut NSError| {
                if let Some(error) = unsafe { error.as_ref() } {
                    sender.send(Event::Authorized(NotificationAuthorized {
                        result: Err(error.retain()),
                    }));
                    return;
                }

                sender.send(Event::Authorized(NotificationAuthorized {
                    result: Ok(granted.as_bool()),
                }));
            }),
        );
    }

    fn send(&self, event: Event) {
        if let Err(error) = self.0.send(event) {
            error!(error = ?error.0, "failed sending event");
        }
    }
}
