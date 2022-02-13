# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### New Features

 - <csr-id-e24957fc191476184816fe5dee249f691170d4ae/> enable use_router
 - <csr-id-29ed7ebece26e9d53925af55f2f34a8fd8241405/> connect an onchange listener
 - <csr-id-d2372717bd01fcff50af0572360e3f763d4c869d/> flatten props attrs

### Bug Fixes

 - <csr-id-5ee9d6c4348a2f51adac827f715fb138918d1dc6/> attach router listener to subscriber list
 - <csr-id-a21e7d4dd168129da06f535f9dc4b1de724617cb/> use_route should subscribe to changes to the route

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 18 commits contributed to the release over the course of 14 calendar days.
 - 7 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #169 from DioxusLabs/jk/router-userouter ([`3509602`](https://github.comgit//DioxusLabs/dioxus/commit/3509602c0bcd327a33bc8c95896775e24751da1a))
    - enable use_router ([`e24957f`](https://github.comgit//DioxusLabs/dioxus/commit/e24957fc191476184816fe5dee249f691170d4ae))
    - add docs to router UseRouteListener ([`79e0993`](https://github.comgit//DioxusLabs/dioxus/commit/79e09934aa685d03d6e0b323723bc1cd537d74d9))
    - Merge pull request #166 from DioxusLabs/jk/default-assets-desktop ([`ccbb955`](https://github.comgit//DioxusLabs/dioxus/commit/ccbb955b7b24bd4e1c5aa40e813a2872ae474a69))
    - rustfmt ([`9da46eb`](https://github.comgit//DioxusLabs/dioxus/commit/9da46eb7bc207997ca7779c58fcb2a9645dfa9d0))
    - Make log message in Link component trace level, not debug ([`72c6bb3`](https://github.comgit//DioxusLabs/dioxus/commit/72c6bb3d0b7253f084f7e3bcf55d458cb4adeedb))
    - attach router listener to subscriber list ([`5ee9d6c`](https://github.comgit//DioxusLabs/dioxus/commit/5ee9d6c4348a2f51adac827f715fb138918d1dc6))
    - connect an onchange listener ([`29ed7eb`](https://github.comgit//DioxusLabs/dioxus/commit/29ed7ebece26e9d53925af55f2f34a8fd8241405))
    - use_route should subscribe to changes to the route ([`a21e7d4`](https://github.comgit//DioxusLabs/dioxus/commit/a21e7d4dd168129da06f535f9dc4b1de724617cb))
    - Merge pull request #95 from DioxusLabs/jk/filedragindrop ([`ca0dd4a`](https://github.comgit//DioxusLabs/dioxus/commit/ca0dd4aa7192d483a195d420363f39d771f3e471))
    - Fix various typos and grammar nits ([`9e4ec43`](https://github.comgit//DioxusLabs/dioxus/commit/9e4ec43b1e78d355c56a38e4c092170b2b01b20d))
    - flatten props attrs ([`d237271`](https://github.comgit//DioxusLabs/dioxus/commit/d2372717bd01fcff50af0572360e3f763d4c869d))
    - Merge pull request #108 from DioxusLabs/jk/fstring-component-fields ([`f4132d1`](https://github.comgit//DioxusLabs/dioxus/commit/f4132d1874f7495049fac23ba0a022ac137ad74f))
    - Enable clippy ([`b6903bf`](https://github.comgit//DioxusLabs/dioxus/commit/b6903bf558bc7a3d0fe6794a137c44fca0957d11))
    - Merge pull request #138 from mrxiaozhuox/master ([`8c7473d`](https://github.comgit//DioxusLabs/dioxus/commit/8c7473d1943dd133f388ec36116c9d8295861b97))
    - Add a warning when Link it called outside of a Router context ([`6408058`](https://github.comgit//DioxusLabs/dioxus/commit/64080588d02c54a1d380116cbecdd17de16d2392))
    - Merge pull request #133 from mrxiaozhuox/master ([`887f69d`](https://github.comgit//DioxusLabs/dioxus/commit/887f69d5b47bdcde4fe0eab094c0cd0de23e4f3f))
    - Fix handling of re-renders in the Router ([`81c094e`](https://github.comgit//DioxusLabs/dioxus/commit/81c094ed29fcdc5c6099492dd6ab09a59b79252c))
</details>

## v0.1.0 (2022-01-08)

### Documentation

 - <csr-id-4de16c4779648e591b3869b5df31271ae603c812/> update local examples and docs to support new syntaxes
 - <csr-id-e4c06ce8e893779d2aad0883a1bb27d193bc5985/> update cargo tomls

### New Features

 - <csr-id-8acdd2ea830b995b608d8bac2ef527db8d40e662/> it compiles once more
 - <csr-id-427b126bc17336d5d14d56eb7fddb8e07752495f/> add prevent default attribute and upgrade router
 - <csr-id-420a30e5d432722e9da16311deb6aa60ea46b0cb/> overhaul examples and clean things up
 - <csr-id-a4f280d16399205c638033bf9beb858e478e98ff/> more API updates

### Bug Fixes

 - <csr-id-58106a55291e90661e8901dd8900434c20a75576/> router version
 - <csr-id-52c7154897111b570918127ffe3285bb1d5951a0/> really big bug around hooks
 - <csr-id-a33f7701fcf5f917fea8719253650b5ad92554fd/> tags
 - <csr-id-868f6739d2b2c5f2ace0c5240cff8008901e818c/> keyword length
 - <csr-id-bd341f5571580cdf5e495379b49ca988fd9211c3/> tests
 - <csr-id-75fa7b4aa672a8a10afcd11016a1b80e0e6f0f02/> make tests pass
 - <csr-id-3dc0e59876f5aba88ed26f1bbd692820f239d4b0/> readme and examples syntax

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 50 commits contributed to the release over the course of 352 calendar days.
 - 35 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release dioxus-hooks v0.1.6, dioxus-mobile v0.0.3, dioxus-router v0.1.0, dioxus-ssr v0.1.2, dioxus-web v0.0.4, dioxus v0.1.7 ([`b804c69`](https://github.comgit//DioxusLabs/dioxus/commit/b804c691d5ade4776390bb3d334cc9cd8efa4a49))
    - More WIP router implementation ([`e06eac1`](https://github.comgit//DioxusLabs/dioxus/commit/e06eac1ce5c6bb0a6680482574d82b16a141a626))
    - Implement UseRoute segment method ([`c9408da`](https://github.comgit//DioxusLabs/dioxus/commit/c9408da7310423b1676fab6d41635f9a8000d89e))
    - Release dioxus-html v0.1.4, dioxus-desktop v0.1.5, dioxus-hooks v0.1.6, dioxus-mobile v0.0.3, dioxus-router v0.1.0, dioxus-ssr v0.1.2, dioxus-web v0.0.4, dioxus v0.1.7 ([`a36dab7`](https://github.comgit//DioxusLabs/dioxus/commit/a36dab7f45920acd8535a69b4aa3695f3bb92111))
    - Implement router matching for path parameters ([`f8a7e1c`](https://github.comgit//DioxusLabs/dioxus/commit/f8a7e1cd8255fd7d0116384247e0e305bb73bf3d))
    - Commit WIP on router ([`3c6142f`](https://github.comgit//DioxusLabs/dioxus/commit/3c6142fb9d8b5f715adf0fb30c2f3534b9ecd923))
    - Add more trace messages to the RouterService code ([`3a5b417`](https://github.comgit//DioxusLabs/dioxus/commit/3a5b417ad1639a31e9aac9f41b05d8b3f074b128))
    - Release dioxus-core v0.1.7, dioxus-core-macro v0.1.6, dioxus-html v0.1.4, dioxus-desktop v0.1.5, dioxus-hooks v0.1.6, dioxus-mobile v0.0.3, dioxus-router v0.1.0, dioxus-ssr v0.1.2, dioxus-web v0.0.4, dioxus v0.1.7 ([`40d1f85`](https://github.comgit//DioxusLabs/dioxus/commit/40d1f85d0c3e2c9fd23c08840cca9f459d4e4307))
    - Fix typo in RouterService struct's "registered_routes" field name ([`d367e0f`](https://github.comgit//DioxusLabs/dioxus/commit/d367e0f89f1b31484bde42efaf10c3531aed1d64))
    - Add title prop to Link ([`e22ba5b`](https://github.comgit//DioxusLabs/dioxus/commit/e22ba5b1e52fe00e08e0b6ac0abae29b6068b8f0))
    - add prevent default attribute and upgrade router ([`427b126`](https://github.comgit//DioxusLabs/dioxus/commit/427b126bc17336d5d14d56eb7fddb8e07752495f))
    - memoize dom in the prescence of identical components ([`cb2782b`](https://github.comgit//DioxusLabs/dioxus/commit/cb2782b4bb34cdaadfff590bfee930ae3ac6536c))
    - bump all versions ([`4f92ba4`](https://github.comgit//DioxusLabs/dioxus/commit/4f92ba41602d706449c1bddabd49829873ee72eb))
    - tests ([`bd341f5`](https://github.comgit//DioxusLabs/dioxus/commit/bd341f5571580cdf5e495379b49ca988fd9211c3))
    - switch to log tracing ([`e2a6454`](https://github.comgit//DioxusLabs/dioxus/commit/e2a6454527cb81d24f7bd2a097beb644f34e3c2d))
    - make tests pass ([`75fa7b4`](https://github.comgit//DioxusLabs/dioxus/commit/75fa7b4aa672a8a10afcd11016a1b80e0e6f0f02))
    - overhaul examples and clean things up ([`420a30e`](https://github.comgit//DioxusLabs/dioxus/commit/420a30e5d432722e9da16311deb6aa60ea46b0cb))
    - remove runner on hook and then update docs ([`d156045`](https://github.comgit//DioxusLabs/dioxus/commit/d1560450bac55f9566e00e00ea405bd1c70b57e5))
    - arbitrary expressions excepted without braces ([`4c85bcf`](https://github.comgit//DioxusLabs/dioxus/commit/4c85bcfdc84184b4fd0fb9317ba31fe569884890))
    - add more svg elements, update readme ([`ad4a0eb`](https://github.comgit//DioxusLabs/dioxus/commit/ad4a0eb3191cefcad3c570517f15f5c0fd7e8687))
    - more API updates ([`a4f280d`](https://github.comgit//DioxusLabs/dioxus/commit/a4f280d16399205c638033bf9beb858e478e98ff))
    - readme and examples syntax ([`3dc0e59`](https://github.comgit//DioxusLabs/dioxus/commit/3dc0e59876f5aba88ed26f1bbd692820f239d4b0))
    - upgrade to new version of dioxus core. ([`cda759c`](https://github.comgit//DioxusLabs/dioxus/commit/cda759c659dfc4b1dde17e3896c35525005026df))
    - remove portals completely ([`2fd56e7`](https://github.comgit//DioxusLabs/dioxus/commit/2fd56e76192bc70d5503bfcd6b4127d383dd082c))
    - miri stress tets ([`934de21`](https://github.comgit//DioxusLabs/dioxus/commit/934de21dd673b1b79904a3249998427f11428426))
    - go back to noisy lifetime solution ([`8daf7a6`](https://github.comgit//DioxusLabs/dioxus/commit/8daf7a6ed86df72522b089aa2647eea7bee0f3b6))
    - rename fc to component ([`1e4a599`](https://github.comgit//DioxusLabs/dioxus/commit/1e4a599d14af85a2d1c29a442dd489f8dc8df321))
    - docs ([`8814977`](https://github.comgit//DioxusLabs/dioxus/commit/8814977eeebe06748a3b9677a8070e42a037ebd7))
    - prepare to change our fragment pattern. Add some more docs ([`2c3a046`](https://github.comgit//DioxusLabs/dioxus/commit/2c3a0464264fa11e8100df025d863931f9606cdb))
    - really big bug around hooks ([`52c7154`](https://github.comgit//DioxusLabs/dioxus/commit/52c7154897111b570918127ffe3285bb1d5951a0))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.1 ([`2b92837`](https://github.comgit//DioxusLabs/dioxus/commit/2b928372fb1b74a4d4e220ff3d798bb7e52f79d2))
    - it compiles once more ([`8acdd2e`](https://github.comgit//DioxusLabs/dioxus/commit/8acdd2ea830b995b608d8bac2ef527db8d40e662))
    - move examples around ([`1e6e5e6`](https://github.comgit//DioxusLabs/dioxus/commit/1e6e5e611b61571f272289adefc9cdd7d59c4399))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`0d480a4`](https://github.comgit//DioxusLabs/dioxus/commit/0d480a4c437d424f0eaff486e510a8fd3f3e6584))
    - updates to router ([`bab21a0`](https://github.comgit//DioxusLabs/dioxus/commit/bab21a0aa1cbf8e6bd95f823e49f53c082e8d6cc))
    - add router ([`d298b62`](https://github.comgit//DioxusLabs/dioxus/commit/d298b626d3ae21a39a8ec4426373369ac94edf9f))
    - keyword length ([`868f673`](https://github.comgit//DioxusLabs/dioxus/commit/868f6739d2b2c5f2ace0c5240cff8008901e818c))
    - docs and router ([`a5f05d7`](https://github.comgit//DioxusLabs/dioxus/commit/a5f05d73acc0e47b05cff64a373482519414bc7c))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`b32665d`](https://github.comgit//DioxusLabs/dioxus/commit/b32665d7212a5b9a3e21cb7af7abba63ae399fac))
    - tags ([`a33f770`](https://github.comgit//DioxusLabs/dioxus/commit/a33f7701fcf5f917fea8719253650b5ad92554fd))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`3a706ac`](https://github.comgit//DioxusLabs/dioxus/commit/3a706ac4168db137723bea90d7a0058190adfc3c))
    - update cargo tomls ([`e4c06ce`](https://github.comgit//DioxusLabs/dioxus/commit/e4c06ce8e893779d2aad0883a1bb27d193bc5985))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`270dfc9`](https://github.comgit//DioxusLabs/dioxus/commit/270dfc9590b2354d083ea8da5cc0e1a1497d30e0))
    - router version ([`58106a5`](https://github.comgit//DioxusLabs/dioxus/commit/58106a55291e90661e8901dd8900434c20a75576))
    - bump versions ([`0846d93`](https://github.comgit//DioxusLabs/dioxus/commit/0846d93d41c27464ca271757c6f24a2cef8fb997))
    - update local examples and docs to support new syntaxes ([`4de16c4`](https://github.comgit//DioxusLabs/dioxus/commit/4de16c4779648e591b3869b5df31271ae603c812))
    - rename ctx to cx ([`81382e7`](https://github.comgit//DioxusLabs/dioxus/commit/81382e7044fb3dba61d4abb1e6086b7b29143116))
    - more work on updating syntad ([`47e8960`](https://github.comgit//DioxusLabs/dioxus/commit/47e896038ef3655566f3eda83d1d2adfefbc8862))
    - massive changes to definition of components ([`508c560`](https://github.comgit//DioxusLabs/dioxus/commit/508c560320d78730fa058156421523ffa5695d9d))
    - add router ([`6aeea9b`](https://github.comgit//DioxusLabs/dioxus/commit/6aeea9b79081a2a711d8eee7e6edd299caf379ef))
</details>

