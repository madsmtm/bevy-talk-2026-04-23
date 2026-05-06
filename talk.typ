#import "@preview/touying:0.7.1": *
#import "@preview/fletcher:0.5.8" as fletcher: diagram, edge, node
#import themes.simple: *

#show: simple-theme.with(
  aspect-ratio: "16-9",
  config-info(
    title: [Bevy on iOS],
    author: [Mads Marquart / \@madsmtm],
    date: datetime(year: 2026, month: 4, day: 23),
  ),
  config-common(
    show-notes-on-second-screen: if sys.inputs.at("present", default: "0") == "1" {
      right
    } else {
      none
    },
  ),
)

#set text(font: "Helvetica Neue")
#set grid(gutter: 50pt)
#show link: underline

#show raw.where(block: false): set text(font: "Monaco")
#show raw.where(block: false): highlight.with(fill: luma(245), extent: 1pt)

#show quote: set text(style: "italic")


= Bevy on iOS

=== with Mads Marquart / \@madsmtm

===

Accessing iOS and macOS platform APIs in Rust using `objc2`.

#speaker-note[
  Thanks for coming!

  I'm Mads, Winit maintainer.

  Stephan asked me to do a talk to make it easier for you to get started with `objc2`.
]

== Contents

1. Platform APIs?

2. Small introduction to Objective-C.

3. Various Objective-C patterns.

4. `'static` callbacks in Bevy.

#speaker-note[
  My goal for today:
  - Give you an introduction to the underlying technology on iOS.
  - Inspire you to create something.

  A few demos throughout.
]


= Platform APIs?

#speaker-note[
  This talk is gonna be about platform APIs.

  If you want to develop for iOS, you'll need to do stuff with the platform APIs.

  I'm gonna say iOS a lot in this talk, but most of this applies to macOS too.
]

== `objc2` in Bevy

#table(
  columns: (auto, auto, auto),
  inset: 10pt,
  align: horizon,
  table.header([*Component*], [*Crate*], [*Underlying platform crates*]),
  [Windowing and Input], [`winit`], [`objc2-ui-kit` / `objc2-app-kit`],
  [Rendering], [`wgpu`], [`objc2-metal`],
  [Audio], [`cpal`], [`objc2-core-audio`],
  [Gamepads], [`gilrs`], [`objc2-io-kit`],
)

#speaker-note[
  To make it a bit more concrete.

  Similar to `windows`, `rustix` and `ndk` crates.

  If you want to fix a bug in Bevy on iOS/macOS, you're gonna run into these cross-platform crates.

  (Some of these not yet, as Bevy hasn't yet updated to the latest versions of these dependencies).
]

== Could also be useful for...

- Notifications / Widgets / similar system GUI stuff
- Neural Engine / built-in ML
- App Store purchases
- Haptic feedback
- Bluetooth / Wi-Fi / NFC / etc.
- TouchID authentication
- Accelerometer / Gyroscope
- ...

#speaker-note[
  Stephan's `bevy_ios_review`.

  TODO.

  I'm not gonna name all of these, you have eyes, you can read.

  Rust ecosystem still underdeveloped, if there was a "are we iOS yet" website, it would be a resounding NO.
]

= Small introduction to Objective-C

Example: Opening a URL in the web browser with `UIKit`.

== What is Objective-C?

- Object-oriented superset of C.

- The language that a large amount of Apple's system APIs are written in.

#speaker-note[
  Swift.

  Thus we must interface with it, it is the platform ABI.
]

== Objective-C declarations

Objetive-C declarations for this:

```objective-c
@interface UIApplication
+ (UIApplication*)sharedApplication;
- (void)openURL:(NSURL*)url
        options:(NSDictionary<NSString*, id> *)options
        completionHandler:(void (^)(BOOL success))completion;
@end
```

#speaker-note[
  There's a lot of smurf naming (`sharedApplication`) (shared namespace). Think Java.

  `-` to represent instance method (`&self`), `+` to do class method.

  Again the `^` to indicate block (closure).
]

== Objective-C's weird method call syntax

```objective-c
UIApplication* app = [UIApplication sharedApplication];

[app openURL:[NSURL URLWithString:@"https://example.com"]
     options:@{}
     completionHandler:^(BOOL success) {
         if (!success) {
             NSLog(@"Failed to open URL");
         }
     }];
```

#speaker-note[
  Square brackets.

  Colons separate arguments.

  The `^` means block, not function. Think closure vs. function pointer.

  Don't ask about the formatting, I have no idea why people want it like this.

  You don't have to know these details, but... well, it does sorta leaks into a lot of the design.

  Mostly, you shouldn't need to care; `objc2` provides high-level bindings to these APIs. But if you're doing some of the lower-level stuff, you might encounter this.
]

== How does that look with `objc2`?

```rust
let app = UIApplication::sharedApplication(mtm);

app.openURL_options_completionHandler(
    &NSURL::URLWithString(ns_string!("https://example.com")),
    &NSDictionary::new(),
    Some(&RcBlock::new(|success| {
        if !success {
            eprintln!("Failed to open URL");
        }
    })),
);
```

#speaker-note[
  This is where `objc2` comes into the picture.

  Auto generation.

  Bit weird method names.

  `&`. Bit more cumbersome around nullability, but hopefully not much worse.

  `RcBlock` stuff.

  (The `webbrowser` crate does this btw).
]

= Demo

#speaker-note[
  Okay, so that was the introduction, now you've at least seen a bit of objc, let's do a demo.
]



= Various Objective-C patterns

Things that are nice to know when using these APIs.

== `MainThreadMarker`

```rust
fn my_system(_: NonSendMarker) {
    let mtm = MainThreadMarker::new().unwrap();
    let app = UIApplication::sharedApplication(mtm);
}
```

#speaker-note[
  Must be accessed on the main thread.
]

== Delegates

```rust
define_class!(
    #[unsafe(super(NSObject))]
    struct MyAppDelegate;

    unsafe impl UIApplicationDelegate for MyAppDelegate {
        #[unsafe(method(applicationWillTerminate:))]
        fn applicationWillTerminate(
            &self, application: &UIApplication,
        ) { /* .. */ }
    }
);
```

#speaker-note[
  Objective-C also has a concept of "protocols", basically traits.

  A common kind of protocol is "delegate" protocols.

  Basically, you have to implement a "trait" (protocol) for a "struct" (class).

  Weak ptr.

  Why this wrapping? Technical reasons.

  Worst part is that `rust-analyzer` doesn't work that well here.
]

== Memory management

`Retained` ≈ `Arc`, everything is refererence-counted.

```rust
struct Foo {
    app: Retained<UIApplication>,
}

fn get_foo(app: &UIApplication) -> Foo {
    Foo { app: app.retain() }
}

foo.app.doThing(...);
```

#speaker-note[
  Thing that often comes up is that you need to store the references somewhere.

  You can do so by "retaining" them, which is the same as cloning an `Arc`.
]

== Completion handlers

```rust
fn doWithCompletionHandler(
    &self,
    completion: &Block<'static, fn(*mut NSError)>,
);

thing.doWithCompletionHandler(&RcBlock::new(|error| {
    if let Some(error) = unsafe { error.as_ref() } {
        eprintln!("failed xyz: {error}");
    }
    // Handle result.
}));
```

#speaker-note[
  We already saw this one, it's common to have some sort of "completion handler", which is Objective-C's way of doing `async`, it's essentially a closure that's run at some later point in time.

  Pointer is currently required because of coherence (https://github.com/rust-lang/rust/issues/56105).
]

== `NSNotificationCenter`

```rust
let center = NSNotificationCenter::defaultCenter();
let observer = center.addObserverForName_object_queue_usingBlock(
    Some(UIContentSizeCategoryDidChangeNotification),
    None, // No sender filter.
    None, // No queue, run on posting thread's runloop.
    &RcBlock::new(|notification: NonNull<NSNotification>| {
        let notification = unsafe { notification.as_ref() };
        // ...
    }),
);
```

#speaker-note[
  Notifications are another mechanism, they're usually delivered as a result of something global happening on the system that you need to listen to.
]


= A wackier demo

Playing a game within UI notifications.

#speaker-note[
  Speaking of notifications, now for a different kind of notification: actual notifications.

  It's bad, don't @ me, I'm not a gamedev.
]


= `'static` callbacks in Bevy

== The problem

How do you go from:

```rust
pub fn open(
    url: &str,
    callback: impl FnOnce(bool) + 'static,
);
```

To something that feels natural in Bevy?

#speaker-note[
  A problem I encountered was this.

  I imagine `async` has much the same problem?
]

== My "solution"

```rust
#[derive(Message)]
pub struct Opened {
    pub url: String,
    pub success: bool,
}
```

#speaker-note[
  My solution was to create a `Message` struct.
]

---

```rust
#[derive(Resource)]
pub struct Browser(mpsc::Sender<Opened>);
impl Browser {
    pub fn open(&self, url: &str) {
        let sender = self.0.clone();
        let url = url.to_string();
        open(&url, move |success| {
            sender.send(Opened { url, success }).unwrap();
        });
    }
}
```

#speaker-note[
  Send that across a channel in the callback.
]

---

```rust
// Inside `Plugin::build`:
let mut receiver = SyncCell::new(receiver);
app.add_systems(
    PreUpdate,
    move |mut writer: MessageWriter<Opened>| {
        for msg in receiver.get().try_iter() {
            writer.write(msg);
        }
    }
);
```

#speaker-note[
  Check for new messages once every iteration of the event loop, and write it to a message queue.
]

---

```rust
fn setup(browser: Res<Browser>) {
    browser.open("http://example.com");
}

fn handle_opened(mut reader: MessageReader<Opened>) {
    for Opened { url, success } in reader.read() {
        if !success {
            eprintln!("failed opening {url:?}");
        }
    }
}
```

#speaker-note[
  And then users would be able to read from the message queue.

  Idk maybe there's some clean way to map it as `Component` state instead? Though that seems like it'd require making the component immutable?

  I'm sure that during the Q&A, someone can tell me all about how there's a better way to do this.

  Especially curious how would you do error handling properly here?
]


= An actually useful demo

Auto text size for accessibility.

#speaker-note[
  iOS has several accessibility features, one of them is content size for people with poor eyesight.

  `Text2d`.

  Should probably be in Bevy itself, but we aren't there yet.
]


= Thanks for listening!

GitHub: https://github.com/madsmtm/objc2

Matrix: https://matrix.to/#/#objc2-users:matrix.org


#speaker-note[
  Hopefully you've gotten a bit of insight into.

  I'd like to encourage you to write wrapper crates for making these things easier (and more cross-platform) to use.

  If you're interested in learning more, feel free to contact me, I hang out at these places, and the Bevy Discord `mac` / `ios` channels.

  Winit ad?
]


= The hidden slides


== How would we call this from Rust?

```objective-c
void open_url(
    const char* url,
    void (*callback)(void*, bool),
    void* context,
) {
    UIApplication* app = [UIApplication sharedApplication];
    [app openURL: /* ... */
         completionHandler:^(BOOL success) {
             callback(context, !!success);
         }];
}
```

#speaker-note[
  One way would be to write and expose a C wrapper.
]

---

```rust
// build.rs

fn main() {
    println!("cargo::rerun-if-changed=src/open_url.m");
    cc::Build::new()
        .file("src/open_url.m")
        .arg("-fobjc-arc")
        .compile("open_url");
}
```

#speaker-note[
  Which we'd then need to build in a build script.
]

---

```rust
unsafe extern "C" {
    fn open_url(
        url: *const c_char,
        callback: extern "C" fn(*mut c_void, bool),
        context: *mut c_void,
    );
}
pub fn open(url: &str, callback: impl FnOnce() + 'static) {
    let callback = Box::new(callback);
    open_url(CString::new(url).unwrap().as_ptr(), /* ... */);
}
```

#speaker-note[
  And then expose in a more Rusty API.

  Here's where I would ask "how many of you have seen this kind of thing before", but, well, I can't see if you raise your hands, so that'd be silly.
]

== Problems?

- It's cumbersome
- It's inefficient
- It's incomplete
- It's untenable

#speaker-note[
  Look. I _hate_ this.

  - Implementation is scattered across three files not including the system header.

  - Cross lang LTO, we call `[UIApplication sharedApplication]` on every call, string allocation, boxing of callback etc.

  - Doesn't support `options`, we'd have to expand it to include that.

  - To do this for the >9000 methods that UIKit exposes.
]
