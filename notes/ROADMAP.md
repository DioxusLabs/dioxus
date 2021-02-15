# Road map
This release map gives a sense of the release cadence and features per update going forward into the future. PRs are required to be squashed before merging. For each point release, we save a branch on master (0.1, 0.2, 0.3, master). Eventually, we'll remove these in favor of higher point releases when dioxus is stabilized. Any live PRs will be merged into the dev branch.

Until 0.3, Dioxus will be in stealth mode. The goal is to launch with a bountiful feature set and a cohesive API before OSS tears it apart :). Once LiveView is ready, then Dioxus will launch completely with a beta service for LiveHost.

## v0.1: Bare Necessities
> Enable ergonomic and performant webapps
---
Dioxus Core
- Lifecycles for components
- Internal event system
- Diffing
- Patching

Html macro
- special formatting
- closure handlers
- child handlers
- iterator handlers
  
Dioxus web
- a
  
Dioxus CLI 
- Develop
- Bundle
- Test
  
Server-side-rendering
- Write nodes to string
- Integration with tide, Actix, warp

Dioxus WebView (desktop)
- One-file setup for desktop apps
- Integration with the web browser for rapid development

## v0.2: Bread and butter
> Complex apps? CHECK
---
State management
- Dioxus-Reducer as the blessed redux alternative
  - Includes thunks and reducers (async dispatches)
- Dioxus-Dataflow as the blessed recoil alternative
  - The hip, new approach for granular state

Dioxus CLI
- Visual tool?
- Asset bundling service
  
Dioxus DevTools integration with the web
- Basic support for pure liveview/webview 


## v0.3: Superpowers
> Enable LiveView for fullstack development
---
Dioxus LiveView
 - Custom server built on Actix (or something fast) 
 - Ergonomic builders
 - Concurrent system built into dioxus core

Dioxus iOS
 - Initial support via webview
 - Look into native support based on how Flutter/SwiftUI works

Dioxus Android


## v0.4: Community 
> Foster the incoming community
