# Dioxus-Desktop

Because the host VirtualDOM is running in its own native process, native applications can unlock their full potential. Dioxus-Desktop is designed to be a 100% rust alternative to ElectronJS without the memory overhead or bloat of ElectronJS apps.

By bridging the native process, desktop apps can access full multithreading power, peripheral support, hardware access, and native filesystem controls without the hassle of web technologies. Our goal with this desktop crate is to make it easy to ship both a web and native application, and quickly see large performance boosts without having to re-write the whole stack. As the dioxus ecosystem grows, we hope to see 3rd parties providing wrappers for storage, offline mode, etc that supports both web and native technologies.
