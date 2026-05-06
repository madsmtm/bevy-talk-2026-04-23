use std::cell::Cell;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

use bevy::app::{App, Plugin, PreUpdate};
use bevy::ecs::resource::Resource;
use bevy::ecs::system::ResMut;
use bevy::log::debug;
use block2::RcBlock;
use dispatch2::MainThreadBound;
use objc2::{ClassType, MainThreadMarker, msg_send, rc::Retained, runtime::ProtocolObject};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObjectProtocol, NSString};
use objc2_ui_kit::{
    UIApplication, UIContentSizeCategory, UIContentSizeCategoryAccessibilityExtraExtraExtraLarge,
    UIContentSizeCategoryAccessibilityExtraExtraLarge,
    UIContentSizeCategoryAccessibilityExtraLarge, UIContentSizeCategoryAccessibilityLarge,
    UIContentSizeCategoryAccessibilityMedium, UIContentSizeCategoryDidChangeNotification,
    UIContentSizeCategoryExtraExtraExtraLarge, UIContentSizeCategoryExtraExtraLarge,
    UIContentSizeCategoryExtraLarge, UIContentSizeCategoryExtraSmall, UIContentSizeCategoryLarge,
    UIContentSizeCategoryMedium, UIContentSizeCategoryNewValueKey, UIContentSizeCategorySmall,
    UIContentSizeCategoryUnspecified,
};

/// The preferred content and text size.
///
/// Controlled in `Settings > Accessibility > Display & Text Size > Larger Text`.
#[non_exhaustive]
#[derive(Resource, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PreferredContentSizeCategory {
    ExtraSmall,
    Small,
    Medium,
    Large,
    ExtraLarge,
    ExtraExtraLarge,
    ExtraExtraExtraLarge,
    // Accessibility sizes are even larger
    AccessibilityMedium,
    AccessibilityLarge,
    AccessibilityExtraLarge,
    AccessibilityExtraExtraLarge,
    AccessibilityExtraExtraExtraLarge,
    Unspecified,
}

impl PreferredContentSizeCategory {
    fn parse(s: &NSString) -> Option<Self> {
        Some(if s == unsafe { UIContentSizeCategoryUnspecified } {
            Self::Unspecified
        } else if s == unsafe { UIContentSizeCategoryExtraSmall } {
            Self::ExtraSmall
        } else if s == unsafe { UIContentSizeCategorySmall } {
            Self::Small
        } else if s == unsafe { UIContentSizeCategoryMedium } {
            Self::Medium
        } else if s == unsafe { UIContentSizeCategoryLarge } {
            Self::Large
        } else if s == unsafe { UIContentSizeCategoryExtraLarge } {
            Self::ExtraLarge
        } else if s == unsafe { UIContentSizeCategoryExtraExtraLarge } {
            Self::ExtraExtraLarge
        } else if s == unsafe { UIContentSizeCategoryExtraExtraExtraLarge } {
            Self::ExtraExtraExtraLarge
        } else if s == unsafe { UIContentSizeCategoryAccessibilityMedium } {
            Self::AccessibilityMedium
        } else if s == unsafe { UIContentSizeCategoryAccessibilityLarge } {
            Self::AccessibilityLarge
        } else if s == unsafe { UIContentSizeCategoryAccessibilityExtraLarge } {
            Self::AccessibilityExtraLarge
        } else if s == unsafe { UIContentSizeCategoryAccessibilityExtraExtraLarge } {
            Self::AccessibilityExtraExtraLarge
        } else if s == unsafe { UIContentSizeCategoryAccessibilityExtraExtraExtraLarge } {
            Self::AccessibilityExtraExtraExtraLarge
        } else {
            return None;
        })
    }
}

pub struct PreferredContentSizeCategoryPlugin {
    _observer: MainThreadBound<Cell<Option<Retained<ProtocolObject<dyn NSObjectProtocol>>>>>,
}

impl PreferredContentSizeCategoryPlugin {
    pub fn new() -> Self {
        let mtm = MainThreadMarker::new().expect("this plugin is only usable on the main thread");
        Self {
            _observer: MainThreadBound::new(Cell::default(), mtm),
        }
    }
}

impl Plugin for PreferredContentSizeCategoryPlugin {
    fn build(&self, _app: &mut App) {
        // Do the work in `finish` instead
    }

    // Not ready before UIApplicationMain is run.
    //
    // We could probably handle this by doing the setup inside `PreStartup` instead?
    fn ready(&self, _app: &App) -> bool {
        let _mtm = MainThreadMarker::new().unwrap();
        let app: Option<Retained<UIApplication>> =
            unsafe { msg_send![UIApplication::class(), sharedApplication] };
        app.is_some()
    }

    fn finish(&self, app: &mut App) {
        let mtm = MainThreadMarker::new().unwrap();
        let initial = UIApplication::sharedApplication(mtm).preferredContentSizeCategory();
        let initial = PreferredContentSizeCategory::parse(&initial)
            .unwrap_or(PreferredContentSizeCategory::Unspecified);
        app.insert_resource(initial);

        // Register an observer and send information about new.
        let current = Arc::new(Mutex::new(initial));
        let current_clone = Arc::clone(&current);

        let center = NSNotificationCenter::defaultCenter();
        let block = RcBlock::new(move |notification: NonNull<NSNotification>| {
            let notification = unsafe { notification.as_ref() };
            let new = notification
                .userInfo()
                .unwrap()
                .objectForKey(unsafe { UIContentSizeCategoryNewValueKey })
                .unwrap()
                .downcast::<UIContentSizeCategory>()
                .unwrap();
            let new = PreferredContentSizeCategory::parse(&new)
                .unwrap_or(PreferredContentSizeCategory::Unspecified);

            debug!("got notification: {new:?}");

            *current.lock().unwrap() = new;
            // TODO: Somehow wake the app? Or is it already gonna be awake?
        });
        let observer = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(UIContentSizeCategoryDidChangeNotification),
                None, // No sender filter
                None, // No queue, run on posting thread (i.e. the main thread)
                &block,
            )
        };
        self._observer.get(mtm).set(Some(observer));

        app.add_systems(
            PreUpdate,
            move |mut old: ResMut<PreferredContentSizeCategory>| {
                let new = *current_clone.lock().unwrap();
                if *old != new {
                    *old = new;
                }
            },
        );
    }
}
