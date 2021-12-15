# Dioxus-Desktop

This crate provides an ergonomic API for Dioxus to build desktop apps.

```rust
fn main() {
    dioxus::desktop::launch(App)
}

static App: Component<()> = |cx| {
    let (count, set_count) = use_state(&cx, || 0);

    cx.render(rsx!(
        WebviewWindow {
            onclose: move |e| log::info!("save our counter state to disk"),
            div {
                h1 { "Dioxus Desktop Demo" }
                p { "Count is {count}"}
                button { onclick: move |_| count += 1}
            }
        }
    ))
}
```

Window management, system trays, notifications, and other desktop-related functionality is managed using the declarative Dioxus API, making it easy to add new features without having to jump through hoops.

## Features
- Your rust code runs natively and under a Tokio runtime
- Declarative application management (dedicated components for windows, models, handlers, task tray, etc)
- Cross platform (runs on Mac, Linux, Windows, etc and mobile through the dioxus-mobile sub crate)

## Managing Windows
Managing windows is done by simply rendering content into a `WebviewWindow` component. 

```rust
static App: Component<()> = |cx| {
    rsx!(cx, WebviewWindow { "hello world" } )
}
```
This will create a new window with only the content "hello world". As this crate matures, we'll have new types of windows for different functionality.

## Managing Notifications 
Notifications also use a declarative approach. Sending a notification has never been easier!

The api has been somewhat modeled after https://github.com/mikaelbr/node-notifier

```rust
static Notifications: Component<()> = |cx| {
    cx.render(rsx!(
        Notification {
            title: "title"
            subtitle: "subtitle"
            message: "message"
            sound: "Basso"
            icon: "Terminal"
            contentImage: "image.png"
            open: "https://github.com"
            wait: true,
            timeout: 5,
            closeLabel: "Cancel"
            actions: ["send", "receive"]
            dropdownLabel: "messaging"
            reply: true

            onclose: move |e| {}
            onreply: move |e| {}
            ondropdownselected: move |e| {}
            ontimeout: move |e| {}
            onerror: move |e| {}
        }
    ))
}

```

## App Tray
Dioxus Desktop supports app trays, which can be built with native menu groups or with a custom window.

```rust
static Tray: Component<()> = |cx| {
    cx.render(rsx!(
        GlobalTray {
            MenuGroup {
                MenuGroupItem { title: "New File", shortcut: "cmd+N", onclick: move |e| {} }
                MenuGroupItem { title: "New Window", shortcut: "shift+cmd+N", onclick: move |e| {} }
            }
        }
    ))
};

// using a builder
static Tray: Component<()> = |cx| {
    let menu = MenuGroup::builder(cx)
        .with_items([
            MenuGroupItem::builder()
                .title()
                .shortcut()
                .onclick(move |e| {}),
            MenuGroupItem::builder()
                .title()
                .shortcut()
                .onclick(move |e| {})
        ]).build();

    rsx!(cx, GlobalTray { rawmenu: menu })
}

// or with a custom window
static Tray: Component<()> = |cx| {
    rsx!(cx, GlobalTray { div { "custom buttons here" } })
};
```

## Menu Bar
Declaring menus is convenient and cross-platform.

```rust
static Menu: Component<()> = |cx| {
    cx.render(rsx!(
        MenuBarMajorItem { title: "File"
            MenuGroup {
                MenuGroupItem { title: "New File", shortcut: "cmd+N", onclick: move |e| {} }
                MenuGroupItem { title: "New Window", shortcut: "shift+cmd+N", onclick: move |e| {} }
            }            
            MenuGroup {
                MenuGroupList { 
                    title: "Open Recent", shortcut: "cmd+N" 
                    MenuGroup {
                        (recent_items.iter().map(|file| rsx!(
                            MenuGroupItem {
                                onclick: move |_| open_file(file),
                                title: "{file}"
                            }
                        )))
                    }
                }
            }
        }        
    ))
};
```

## Building, bundling, etc

and then to create a native .app:

```
dioxus bundle --platform macOS
```

## Goals

Because the host VirtualDOM is running in its own native process, native applications can unlock their full potential. Dioxus-Desktop is designed to be a 100% rust alternative to ElectronJS without the memory overhead or bloat of ElectronJS apps.

By bridging the native process, desktop apps can access full multithreading power, peripheral support, hardware access, and native filesystem controls without the hassle of web technologies. Our goal with this desktop crate is to make it easy to ship both a web and native application, and quickly see large performance boosts without having to re-write the whole stack. As the dioxus ecosystem grows, we hope to see 3rd parties providing wrappers for storage, offline mode, etc that supports both web and native technologies.
