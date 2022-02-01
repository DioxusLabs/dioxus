# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### New Features

 - <csr-id-d2372717bd01fcff50af0572360e3f763d4c869d/> flatten props attrs
 - <csr-id-47bc4e4a44a7d08d4f42102d13f0766d9d6bf358/> add "optional" flag for props

### Bug Fixes

 - <csr-id-90abd9c9a08c165384fae9c1f7c3fd098d512c48/> detection of f-string formatting in components

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 12 calendar days.
 - 3 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #95 from DioxusLabs/jk/filedragindrop ([`ca0dd4a`](https://github.comgit//DioxusLabs/dioxus/commit/ca0dd4aa7192d483a195d420363f39d771f3e471))
    - Fix various typos and grammar nits ([`9e4ec43`](https://github.comgit//DioxusLabs/dioxus/commit/9e4ec43b1e78d355c56a38e4c092170b2b01b20d))
    - flatten props attrs ([`d237271`](https://github.comgit//DioxusLabs/dioxus/commit/d2372717bd01fcff50af0572360e3f763d4c869d))
    - Merge pull request #108 from DioxusLabs/jk/fstring-component-fields ([`f4132d1`](https://github.comgit//DioxusLabs/dioxus/commit/f4132d1874f7495049fac23ba0a022ac137ad74f))
    - detection of f-string formatting in components ([`90abd9c`](https://github.comgit//DioxusLabs/dioxus/commit/90abd9c9a08c165384fae9c1f7c3fd098d512c48))
    - Enable clippy ([`b6903bf`](https://github.comgit//DioxusLabs/dioxus/commit/b6903bf558bc7a3d0fe6794a137c44fca0957d11))
    - Merge pull request #107 from autarch/autarch/half-assed-router ([`8d3ac3f`](https://github.comgit//DioxusLabs/dioxus/commit/8d3ac3ff148aef9d10a393eda453a11c1e882f58))
    - Merge pull request #127 from DioxusLabs/jk/handler-tweak ([`5bce294`](https://github.comgit//DioxusLabs/dioxus/commit/5bce294a86941090660b1cfd87809a2b0b76d2ce))
    - add "optional" flag for props ([`47bc4e4`](https://github.comgit//DioxusLabs/dioxus/commit/47bc4e4a44a7d08d4f42102d13f0766d9d6bf358))
</details>

## v0.1.6 (2022-01-08)

### Documentation

 - <csr-id-cafb7df736e9366c2acd99b5571cd4b7894ea595/> remove all usages of static closure syntax and update readme
 - <csr-id-e4c06ce8e893779d2aad0883a1bb27d193bc5985/> update cargo tomls
 - <csr-id-4de16c4779648e591b3869b5df31271ae603c812/> update local examples and docs to support new syntaxes
 - <csr-id-583fdfa5618e11d660985b97e570d4503be2ff49/> big updates to the reference
 - <csr-id-d9e6d0925b30690212d1d690dfba288f1a694a27/> examples
 - <csr-id-daa9bd82c365763fe240528c7df222d230bce613/> more work on docs

### New Features

 - <csr-id-427b126bc17336d5d14d56eb7fddb8e07752495f/> add prevent default attribute and upgrade router
 - <csr-id-a4f280d16399205c638033bf9beb858e478e98ff/> more API updates
 - <csr-id-b997b8ebbb82b5b9e9119bd2eb25335e2ed009d0/> enable children properly
 - <csr-id-fd93ee89c19b085a04307ef30217170518defa8e/> upgrade syntax
 - <csr-id-2cf90b6903411e42f01a801f89037686194ee068/> pull children out of component definition
 - <csr-id-84fd0c616252bf29cd665782258530032b54d13a/> cleanuup
 - <csr-id-79503f15c5db04fa04575c8735941a2e3a75030b/> full html support
 - <csr-id-9726a065b0d4fb1ede5b53a2ddd58c855e51539f/> massage lifetimes
 - <csr-id-fac42339c272b0e430ebf4f31b6061a0635d3e19/> mutations
 - <csr-id-4a72b3140bd244da602deada1eeecded65ff5848/> amazingly awesome error handling
 - <csr-id-a2c7d17b0595769f60bc1c2bbf7cbe32cec37486/> mvoe away from compound context
 - <csr-id-7dfe89c9581f45a445f17f9fe4bb94e61f67e971/> wire up event delegator for webview
 - <csr-id-4091846934b4b3b2bc03d3ca8aaf7712aebd4e36/> add aria
 - <csr-id-e4cdb645aad800484b19ec35ba1f8bb9ccf71d12/> beaf up the use_state hook
 - <csr-id-7aec40d57e78ec13ff3a90ca8149521cbf1d9ff2/> enable arbitrary body in rsx! macro
 - <csr-id-22e659c2bd7797ca5a822180aca0cb5d950c5287/> namespaced attributes
   this commit adds namespaced attributes. This lets us support attribute groups, and thus, inline styles.
   
   This namespaced attribute stuff is only available for styles at the moment, though it theoretically could be enabled for any other attributes.
 - <csr-id-904b26f7111c3fc66400744ff6192e4b20bf6d74/> add edits back! and more webview support!
   This commit adds a new type - the DomEdit - for serializing the changes made by the diffing machine. The architecture of how DomEdits fit into the cooperative scheduling is still TBD but it will allow us to build change lists without applying them immediately. This is more performant  and allows us to only render parts of the page at a time.
   
   This commit also adds more infrastructure around webview. Dioxus can now run on the web, generate static pages, run in the desktop, and run on mobile, with a large part of thanks to webview.
 - <csr-id-b5e5ef171aa9f8986fb4ab04d793eb63f557c4ae/> two calculator examples
 - <csr-id-73047fe95678d50fcfd62a4ace7c6b406c5304e1/> props memoization is more powerful
   This commit solves the memoization , properly memoizing properties that don't have any generic parameters. This is a rough heuristic to prevent non-static lifetimes from creeping into props and breaking our minual lifetime management.
   
   Props that have a generic parameter are opted-out of the `partialeq` requirement and props *without* lifetimes must implement partialeq. We're going to leave manual disabling of memoization for future work.
 - <csr-id-cfa0927cdd40bc3dba22996018605dbad91d0391/> todomvc

### Bug Fixes

 - <csr-id-c439b0ac7e09f70a04262b7c29938d8c52197b76/> component pass thru events
 - <csr-id-2481cd05c2371b4a23b03d7710598708b1b2e491/> attempt to fix ice
 - <csr-id-bd341f5571580cdf5e495379b49ca988fd9211c3/> tests
 - <csr-id-3dc0e59876f5aba88ed26f1bbd692820f239d4b0/> readme and examples syntax
 - <csr-id-52c7154897111b570918127ffe3285bb1d5951a0/> really big bug around hooks
 - <csr-id-868f6739d2b2c5f2ace0c5240cff8008901e818c/> keyword length
 - <csr-id-a33f7701fcf5f917fea8719253650b5ad92554fd/> tags
 - <csr-id-27d891934a70424b45e6278b7e2baaa2d1b78b35/> use annotation method from rust/58052 to fix closure lifetimes
 - <csr-id-ba9e1dbb8fa24048a6c9ccef8a8722688226a845/> messed up how lifetimes worked, need to render once per component

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 139 commits contributed to the release over the course of 352 calendar days.
 - 123 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release dioxus-core-macro v0.1.6 ([`db0a5bd`](https://github.comgit//DioxusLabs/dioxus/commit/db0a5bd6ec93803cddb3c6fda4172257b9c301c6))
    - Release dioxus-core v0.1.7, dioxus-core-macro v0.1.6, dioxus-html v0.1.4, dioxus-desktop v0.1.5, dioxus-hooks v0.1.6, dioxus-mobile v0.0.3, dioxus-router v0.1.0, dioxus-ssr v0.1.2, dioxus-web v0.0.4, dioxus v0.1.7 ([`40d1f85`](https://github.comgit//DioxusLabs/dioxus/commit/40d1f85d0c3e2c9fd23c08840cca9f459d4e4307))
    - component pass thru events ([`c439b0a`](https://github.comgit//DioxusLabs/dioxus/commit/c439b0ac7e09f70a04262b7c29938d8c52197b76))
    - Merge pull request #74 from mrxiaozhuox/master ([`47056fd`](https://github.comgit//DioxusLabs/dioxus/commit/47056fda4577bcbdaa2a6f63d82eec876e5a5aee))
    - Merge pull request #80 from DioxusLabs/jk/router2dotoh ([`cdc2d8e`](https://github.comgit//DioxusLabs/dioxus/commit/cdc2d8ec6d123245c2ea5f6d10af02b6a6833994))
    - Merge pull request #79 from DioxusLabs/jk/better_rehydration ([`34b0cb5`](https://github.comgit//DioxusLabs/dioxus/commit/34b0cb500bb59b5c9190cd00a46c27609774c327))
    - Merge pull request #78 from higumachan/patch-1 ([`c9aaa96`](https://github.comgit//DioxusLabs/dioxus/commit/c9aaa9643229199ff061140dfbb323b50d035fd3))
    - add prevent default attribute and upgrade router ([`427b126`](https://github.comgit//DioxusLabs/dioxus/commit/427b126bc17336d5d14d56eb7fddb8e07752495f))
    - memoize dom in the prescence of identical components ([`cb2782b`](https://github.comgit//DioxusLabs/dioxus/commit/cb2782b4bb34cdaadfff590bfee930ae3ac6536c))
    - Fix typo "misisng" that occurred in several error messages ([`51c177b`](https://github.comgit//DioxusLabs/dioxus/commit/51c177be9e68d55c0b74f9a53d86a0d59d83c60e))
    - Fix reference to "html!" macro in "rsx!" macro docs ([`706b0e2`](https://github.comgit//DioxusLabs/dioxus/commit/706b0e21a50c66e1056f1338e4249b3faf1673ab))
    - new versions of everything ([`4ea5c99`](https://github.comgit//DioxusLabs/dioxus/commit/4ea5c990d72b1645724ab0a88ffea2baf28e2835))
    - attempt to fix ice ([`2481cd0`](https://github.comgit//DioxusLabs/dioxus/commit/2481cd05c2371b4a23b03d7710598708b1b2e491))
    - tests ([`bd341f5`](https://github.comgit//DioxusLabs/dioxus/commit/bd341f5571580cdf5e495379b49ca988fd9211c3))
    - update core, core-macro, and html ([`f9b9bb9`](https://github.comgit//DioxusLabs/dioxus/commit/f9b9bb9c0c2c55f55d2d6860e3d2d986debd6412))
    - remove all usages of static closure syntax and update readme ([`cafb7df`](https://github.comgit//DioxusLabs/dioxus/commit/cafb7df736e9366c2acd99b5571cd4b7894ea595))
    - remove runner on hook and then update docs ([`d156045`](https://github.comgit//DioxusLabs/dioxus/commit/d1560450bac55f9566e00e00ea405bd1c70b57e5))
    - arbitrary expressions excepted without braces ([`4c85bcf`](https://github.comgit//DioxusLabs/dioxus/commit/4c85bcfdc84184b4fd0fb9317ba31fe569884890))
    - polish some more things ([`1496102`](https://github.comgit//DioxusLabs/dioxus/commit/14961023f927b3a8bde83cfc7883aa8bfcca9e85))
    - more API updates ([`a4f280d`](https://github.comgit//DioxusLabs/dioxus/commit/a4f280d16399205c638033bf9beb858e478e98ff))
    - upgrade hooks ([`b3ac2ee`](https://github.comgit//DioxusLabs/dioxus/commit/b3ac2ee3f76549cd1c7b6f9eee7e3382b07d873c))
    - bump macro crate ([`639f9f5`](https://github.comgit//DioxusLabs/dioxus/commit/639f9f53224a1a8a6699d6bbb5f46f328f645031))
    - readme and examples syntax ([`3dc0e59`](https://github.comgit//DioxusLabs/dioxus/commit/3dc0e59876f5aba88ed26f1bbd692820f239d4b0))
    - upgrade to new version of dioxus core. ([`cda759c`](https://github.comgit//DioxusLabs/dioxus/commit/cda759c659dfc4b1dde17e3896c35525005026df))
    - enable children properly ([`b997b8e`](https://github.comgit//DioxusLabs/dioxus/commit/b997b8ebbb82b5b9e9119bd2eb25335e2ed009d0))
    - docs ([`8814977`](https://github.comgit//DioxusLabs/dioxus/commit/8814977eeebe06748a3b9677a8070e42a037ebd7))
    - polish ([`8bf57dc`](https://github.comgit//DioxusLabs/dioxus/commit/8bf57dc21dfbcbae5b95650203b68d3f41227652))
    - really big bug around hooks ([`52c7154`](https://github.comgit//DioxusLabs/dioxus/commit/52c7154897111b570918127ffe3285bb1d5951a0))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.1 ([`2b92837`](https://github.comgit//DioxusLabs/dioxus/commit/2b928372fb1b74a4d4e220ff3d798bb7e52f79d2))
    - rename ([`36d89be`](https://github.comgit//DioxusLabs/dioxus/commit/36d89beb34821694cb0afb546d3b0cb4e01aaae1))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`0d480a4`](https://github.comgit//DioxusLabs/dioxus/commit/0d480a4c437d424f0eaff486e510a8fd3f3e6584))
    - updates to router ([`bab21a0`](https://github.comgit//DioxusLabs/dioxus/commit/bab21a0aa1cbf8e6bd95f823e49f53c082e8d6cc))
    - add router ([`d298b62`](https://github.comgit//DioxusLabs/dioxus/commit/d298b626d3ae21a39a8ec4426373369ac94edf9f))
    - keyword length ([`868f673`](https://github.comgit//DioxusLabs/dioxus/commit/868f6739d2b2c5f2ace0c5240cff8008901e818c))
    - docs and router ([`a5f05d7`](https://github.comgit//DioxusLabs/dioxus/commit/a5f05d73acc0e47b05cff64a373482519414bc7c))
    - upgrade syntax ([`fd93ee8`](https://github.comgit//DioxusLabs/dioxus/commit/fd93ee89c19b085a04307ef30217170518defa8e))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`b32665d`](https://github.comgit//DioxusLabs/dioxus/commit/b32665d7212a5b9a3e21cb7af7abba63ae399fac))
    - tags ([`a33f770`](https://github.comgit//DioxusLabs/dioxus/commit/a33f7701fcf5f917fea8719253650b5ad92554fd))
    - Merge branch 'master' into jk/remove_node_safety ([`db00047`](https://github.comgit//DioxusLabs/dioxus/commit/db0004758c77331cc3b93ea8cf227c060028e12e))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`3a706ac`](https://github.comgit//DioxusLabs/dioxus/commit/3a706ac4168db137723bea90d7a0058190adfc3c))
    - pull children out of component definition ([`2cf90b6`](https://github.comgit//DioxusLabs/dioxus/commit/2cf90b6903411e42f01a801f89037686194ee068))
    - update cargo tomls ([`e4c06ce`](https://github.comgit//DioxusLabs/dioxus/commit/e4c06ce8e893779d2aad0883a1bb27d193bc5985))
    - bubbling in progress ([`a21020e`](https://github.comgit//DioxusLabs/dioxus/commit/a21020ea575e467ba0d608737269fe1b0792dba7))
    - cleanuup ([`84fd0c6`](https://github.comgit//DioxusLabs/dioxus/commit/84fd0c616252bf29cd665782258530032b54d13a))
    - clippy happy on macro ([`e1c858d`](https://github.comgit//DioxusLabs/dioxus/commit/e1c858dda5c937a56f402bfb3e8b90baf34b84f1))
    - remove bump ([`fcc6738`](https://github.comgit//DioxusLabs/dioxus/commit/fcc6738f1703006d7678f31a39bbf6d59464a7e1))
    - fix some bugs around the rsx macro ([`339e450`](https://github.comgit//DioxusLabs/dioxus/commit/339e450027b4a5d2e1317e13863cd1b2e7ab5853))
    - full html support ([`79503f1`](https://github.comgit//DioxusLabs/dioxus/commit/79503f15c5db04fa04575c8735941a2e3a75030b))
    - remove HTML macro and add custom fields ([`9f7eb0f`](https://github.comgit//DioxusLabs/dioxus/commit/9f7eb0f6002156d3e6e14ea2cb24829133b531c5))
    - update local examples and docs to support new syntaxes ([`4de16c4`](https://github.comgit//DioxusLabs/dioxus/commit/4de16c4779648e591b3869b5df31271ae603c812))
    - use annotation method from rust/58052 to fix closure lifetimes ([`27d8919`](https://github.comgit//DioxusLabs/dioxus/commit/27d891934a70424b45e6278b7e2baaa2d1b78b35))
    - worked backwards a bit and got it slightly figured out ([`9ee2bfb`](https://github.comgit//DioxusLabs/dioxus/commit/9ee2bfb010ce90ec97e93e173c31aab281db32c4))
    - massage lifetimes ([`9726a06`](https://github.comgit//DioxusLabs/dioxus/commit/9726a065b0d4fb1ede5b53a2ddd58c855e51539f))
    - book documentation ([`16dbf4a`](https://github.comgit//DioxusLabs/dioxus/commit/16dbf4a6f84103857385fb4b142a718b0ce72118))
    - messed up how lifetimes worked, need to render once per component ([`ba9e1db`](https://github.comgit//DioxusLabs/dioxus/commit/ba9e1dbb8fa24048a6c9ccef8a8722688226a845))
    - major cleanups to scheduler ([`2933e4b`](https://github.comgit//DioxusLabs/dioxus/commit/2933e4bc11b3074c2bde8d76ec55364fca841988))
    - move everything over to a stack dst ([`0e9d5fc`](https://github.comgit//DioxusLabs/dioxus/commit/0e9d5fc5306ab508d5af6999a4064f9b8b48460f))
    - remove warnings on core macero ([`6587224`](https://github.comgit//DioxusLabs/dioxus/commit/6587224debffa8e8d5282dc3f120abbaa96f552b))
    - mutations ([`fac4233`](https://github.comgit//DioxusLabs/dioxus/commit/fac42339c272b0e430ebf4f31b6061a0635d3e19))
    - bottom up dropping ([`f2334c1`](https://github.comgit//DioxusLabs/dioxus/commit/f2334c17be2612d926361686d7d40a57e3ffe9b9))
    - fill out the snippets ([`6051b0e`](https://github.comgit//DioxusLabs/dioxus/commit/6051b0ec86927704451f4ce6cdf8f988e59702ae))
    - amazingly awesome error handling ([`4a72b31`](https://github.comgit//DioxusLabs/dioxus/commit/4a72b3140bd244da602deada1eeecded65ff5848))
    - big updates to the reference ([`583fdfa`](https://github.comgit//DioxusLabs/dioxus/commit/583fdfa5618e11d660985b97e570d4503be2ff49))
    - docs, html! macro, more ([`caf772c`](https://github.comgit//DioxusLabs/dioxus/commit/caf772cf249d2f56c8d0b0fa2737ad48e32c6e82))
    - get keyed diffing compiling ([`0a0be95`](https://github.comgit//DioxusLabs/dioxus/commit/0a0be95c3e58dc065409f02f703b82700c1003f8))
    - changes to scheduler ([`098d382`](https://github.comgit//DioxusLabs/dioxus/commit/098d3821ed89ad38d99077a6556b48a7e91fc3fc))
    - clean up warnings ([`b32e261`](https://github.comgit//DioxusLabs/dioxus/commit/b32e2611e37b17c2371ffb10cf1ac647f017d917))
    - mvoe away from compound context ([`a2c7d17`](https://github.comgit//DioxusLabs/dioxus/commit/a2c7d17b0595769f60bc1c2bbf7cbe32cec37486))
    - wire up event delegator for webview ([`7dfe89c`](https://github.comgit//DioxusLabs/dioxus/commit/7dfe89c9581f45a445f17f9fe4bb94e61f67e971))
    - basic support for scheduled rendering ([`c52af22`](https://github.comgit//DioxusLabs/dioxus/commit/c52af221f755601a9e826ffc2c355def138999d0))
    - solve some issues regarding listeners ([`dfaf5ad`](https://github.comgit//DioxusLabs/dioxus/commit/dfaf5adee164f44a679ab21d730caaab3610e01f))
    - change in cx to cx ([`9971ff2`](https://github.comgit//DioxusLabs/dioxus/commit/9971ff215db6f771b7ec1cae2517c85d47d38622))
    - move things into a "shared" object ([`f644d7c`](https://github.comgit//DioxusLabs/dioxus/commit/f644d7c44159eef091552dcc90acbb151ea76b21))
    - apply formatting ([`a85b8c4`](https://github.comgit//DioxusLabs/dioxus/commit/a85b8c4b6be83f7aba06714b6a1ff0aa5f2ee729))
    - more upgades to html parser ([`22f894e`](https://github.comgit//DioxusLabs/dioxus/commit/22f894e6b98073bffa39f08b890071ffc00b8d49))
    - serious refactor with const generics ([`160d86a`](https://github.comgit//DioxusLabs/dioxus/commit/160d86abbe1b325e3123202aef29025dcd96f4eb))
    - ....sigh..... so the diffing algorithm is robust ([`68ed1c0`](https://github.comgit//DioxusLabs/dioxus/commit/68ed1c04e7e773f9e6c0a5148f0ea89b97b6784e))
    - add aria ([`4091846`](https://github.comgit//DioxusLabs/dioxus/commit/4091846934b4b3b2bc03d3ca8aaf7712aebd4e36))
    - move examples around ([`304259d`](https://github.comgit//DioxusLabs/dioxus/commit/304259d8186d1d34224a74c95f4fd7d14126b499))
    - beaf up the use_state hook ([`e4cdb64`](https://github.comgit//DioxusLabs/dioxus/commit/e4cdb645aad800484b19ec35ba1f8bb9ccf71d12))
    - enable arbitrary body in rsx! macro ([`7aec40d`](https://github.comgit//DioxusLabs/dioxus/commit/7aec40d57e78ec13ff3a90ca8149521cbf1d9ff2))
    - move some examples around ([`98a0933`](https://github.comgit//DioxusLabs/dioxus/commit/98a09339fd3190799ea4dd316908f0a53fdf2413))
    - fix issues with lifetimes ([`a38a81e`](https://github.comgit//DioxusLabs/dioxus/commit/a38a81e1290375cae685f7c49d3745e4298fab26))
    - namespaced attributes ([`22e659c`](https://github.comgit//DioxusLabs/dioxus/commit/22e659c2bd7797ca5a822180aca0cb5d950c5287))
    - groundwork for noderefs ([`c1afeba`](https://github.comgit//DioxusLabs/dioxus/commit/c1afeba1efb1a063705466a14648beee08cacb86))
    - add edits back! and more webview support! ([`904b26f`](https://github.comgit//DioxusLabs/dioxus/commit/904b26f7111c3fc66400744ff6192e4b20bf6d74))
    - enable more diffing ([`e8f29a8`](https://github.comgit//DioxusLabs/dioxus/commit/e8f29a8f8ac56020bee0048021efa52547307a77))
    - two calculator examples ([`b5e5ef1`](https://github.comgit//DioxusLabs/dioxus/commit/b5e5ef171aa9f8986fb4ab04d793eb63f557c4ae))
    - examples ([`d9e6d09`](https://github.comgit//DioxusLabs/dioxus/commit/d9e6d0925b30690212d1d690dfba288f1a694a27))
    - wip ([`952a91d`](https://github.comgit//DioxusLabs/dioxus/commit/952a91d5408aaf789b496f11d01c3b3f7fcf9059))
    - rename ctx to cx ([`81382e7`](https://github.comgit//DioxusLabs/dioxus/commit/81382e7044fb3dba61d4abb1e6086b7b29143116))
    - rethinking stack machine ([`62ae5d3`](https://github.comgit//DioxusLabs/dioxus/commit/62ae5d3bb94cb9ead030ae0b39d9d9bc2b8b4532))
    - more work on docs ([`daa9bd8`](https://github.comgit//DioxusLabs/dioxus/commit/daa9bd82c365763fe240528c7df222d230bce613))
    - some cleanup and documentation ([`517d7f1`](https://github.comgit//DioxusLabs/dioxus/commit/517d7f14957c4dae9fc894bfbdcd00a955d09f20))
    - docs ([`f5683a2`](https://github.comgit//DioxusLabs/dioxus/commit/f5683a23464992ecace463a61414795b5a2c58c8))
    - pre vnodes instead of vnode ([`fe6938c`](https://github.comgit//DioxusLabs/dioxus/commit/fe6938ceb3dba0796ae8bab52ae41248dc0d3650))
    - props memoization is more powerful ([`73047fe`](https://github.comgit//DioxusLabs/dioxus/commit/73047fe95678d50fcfd62a4ace7c6b406c5304e1))
    - merge in some code from the other branch ([`7790750`](https://github.comgit//DioxusLabs/dioxus/commit/7790750349b40055673a0ec16074a0426b84d3b3))
    - move the rsx macro around ([`50c8b93`](https://github.comgit//DioxusLabs/dioxus/commit/50c8b93aade1bfa83a091fb51ee48638507f89b0))
    - massive changes to definition of components ([`508c560`](https://github.comgit//DioxusLabs/dioxus/commit/508c560320d78730fa058156421523ffa5695d9d))
    - move to static props ([`c1fd848`](https://github.comgit//DioxusLabs/dioxus/commit/c1fd848f89b0146581d8e485fa0d4a847387b963))
    - more progress on parity docs. ([`c5089ba`](https://github.comgit//DioxusLabs/dioxus/commit/c5089ba3c5a8daad4de4d6257604011cc87f6ac7))
    - buff the readme and docs ([`3cfa1fe`](https://github.comgit//DioxusLabs/dioxus/commit/3cfa1fe125886787f35905ed9b05340a739bc654))
    - Todomvc in progress ([`b843dbd`](https://github.comgit//DioxusLabs/dioxus/commit/b843dbd3679abf86a34347d87fd4ce5fe9e2aca5))
    - remove old code ([`3de54d0`](https://github.comgit//DioxusLabs/dioxus/commit/3de54d0b5202aca678d485a68ef8de006a63e21b))
    - some code health ([`c28697e`](https://github.comgit//DioxusLabs/dioxus/commit/c28697e1fe3136d1835f2b663715f34aab9f4b17))
    - major overhaul to diffing ([`9810fee`](https://github.comgit//DioxusLabs/dioxus/commit/9810feebf57f93114e3d7faf6de053ac192593a9))
    - todos ([`8c541f6`](https://github.comgit//DioxusLabs/dioxus/commit/8c541f66d5f7ef2286f2cdf9b0496a9c404471f9))
    - todomvc ([`cfa0927`](https://github.comgit//DioxusLabs/dioxus/commit/cfa0927cdd40bc3dba22996018605dbad91d0391))
    - todomvc ([`ce33031`](https://github.comgit//DioxusLabs/dioxus/commit/ce33031519fbbbd207f1dffb75acf62bf59e3c9e))
    - more ergonomics, more examples ([`0bcff1f`](https://github.comgit//DioxusLabs/dioxus/commit/0bcff1f88e4b1a633b7a9b7c6c2e39b8bd3666c4))
    - use rsx! inline! ([`44aad27`](https://github.comgit//DioxusLabs/dioxus/commit/44aad2746c117ba9742c86a53327f4f9e96509e7))
    - building large apps, revamp macro ([`9f7f43b`](https://github.comgit//DioxusLabs/dioxus/commit/9f7f43b6614aaef2d7dded7058e81934f28f5dec))
    - begint to accept iterator types ([`742f150`](https://github.comgit//DioxusLabs/dioxus/commit/742f150eb3eba89913f5a0fabb229e72e2a0a5ee))
    - props now autoderives its own trait ([`b3c96a5`](https://github.comgit//DioxusLabs/dioxus/commit/b3c96a5996f434332813c737bb83ad564d91af5f))
    - staticify? ([`5ad8188`](https://github.comgit//DioxusLabs/dioxus/commit/5ad81885e499bf02ac79e0098f7956d02ee5f2e5))
    - cargo fix to clean up things ([`78d093a`](https://github.comgit//DioxusLabs/dioxus/commit/78d093a9454386397a991bd01e603e4ad554521f))
    - wire up props macro ([`37f5a7a`](https://github.comgit//DioxusLabs/dioxus/commit/37f5a7ad33e272a9e210bf480304d54ff0df0d67))
    - revert FC changes (like the old style). ([`7158bc3`](https://github.comgit//DioxusLabs/dioxus/commit/7158bc3575e180dbe8641549b040e74ae3baf80b))
    - yeet, synthetic somewhat wired up ([`d959806`](https://github.comgit//DioxusLabs/dioxus/commit/d9598066c2679d9d0b9ca0ce1d3f26110a238cd2))
    - remove FC ([`92d9521`](https://github.comgit//DioxusLabs/dioxus/commit/92d9521a73aefb620b354ae5954617109dd06e7e))
    - more cleanup ([`5a9155b`](https://github.comgit//DioxusLabs/dioxus/commit/5a9155b059acc1fb3c8b8accbeca3701ce4f0ab6))
    - add context to builder ([`cf16090`](https://github.comgit//DioxusLabs/dioxus/commit/cf16090838d127354e333dcbc0b06474835b87d6))
    - listeners now have scope information ([`fcd68e6`](https://github.comgit//DioxusLabs/dioxus/commit/fcd68e61d2400628469ba193b009e7bf1fd3acdf))
    - broken, but solved ([`cb74d70`](https://github.comgit//DioxusLabs/dioxus/commit/cb74d70f831b5510f1ee191d91eaff621ffa6256))
    - accept closures directly in handler ([`f225030`](https://github.comgit//DioxusLabs/dioxus/commit/f225030506967415a21f4af0372477cb5224ee7c))
    - wowza got it all working ([`4b8e9f4`](https://github.comgit//DioxusLabs/dioxus/commit/4b8e9f4a125b9d55439d919786f33d9d5df234e8))
    - parse custom rsx syntax ([`da00df6`](https://github.comgit//DioxusLabs/dioxus/commit/da00df66889f4fa2e39651491e08794e1fe78549))
    - update readme and examples ([`ffaf687`](https://github.comgit//DioxusLabs/dioxus/commit/ffaf6878963981860089c2362947bf77a84c9058))
    - add core macro crate ([`6a7bf3f`](https://github.comgit//DioxusLabs/dioxus/commit/6a7bf3f964150bcb8f7ba35ad285dd7deff7955c))
    - add in style crate, and abort any styligng ([`c09b71f`](https://github.comgit//DioxusLabs/dioxus/commit/c09b71f473ceeac7d37cd2b4117786350b6b11b6))
    - remove html crate ([`9dcee01`](https://github.comgit//DioxusLabs/dioxus/commit/9dcee01b335901cf2c80b453b97180e0d2551dc2))
    - add core macro crate ([`9f49ecb`](https://github.comgit//DioxusLabs/dioxus/commit/9f49ecbd95b60deb74c646f3798dfde3542c44be))
    - custom format_args for inlining variables into html templates ([`e4b1f6e`](https://github.comgit//DioxusLabs/dioxus/commit/e4b1f6ea0d0db707cf757dabf8635e9fc91a3e0f))
    - comment out examples and move lifetime in FC type ([`62d4ad5`](https://github.comgit//DioxusLabs/dioxus/commit/62d4ad58787185032100a2d25e79b70f6ec97a3c))
    - include the helper ([`07341d2`](https://github.comgit//DioxusLabs/dioxus/commit/07341d2c65dc61b90587e2e5daadf72ec82623a8))
    - Dioxus-webview ([`9c01736`](https://github.comgit//DioxusLabs/dioxus/commit/9c0173689539210d14847613f9a1694e6cb34506))
    - update fc_macro ([`28ac37a`](https://github.comgit//DioxusLabs/dioxus/commit/28ac37a8b23874c77011a46a11e6b9cbdf79ecdd))
    - dioxus frontend crate ([`4d7ac5b`](https://github.comgit//DioxusLabs/dioxus/commit/4d7ac5bb5d3aa1897c0f6c1f322aca08c0c791f0))
</details>

## v0.1.2 (2021-12-15)

### Documentation

 - <csr-id-4de16c4779648e591b3869b5df31271ae603c812/> update local examples and docs to support new syntaxes
 - <csr-id-583fdfa5618e11d660985b97e570d4503be2ff49/> big updates to the reference
 - <csr-id-d9e6d0925b30690212d1d690dfba288f1a694a27/> examples
 - <csr-id-daa9bd82c365763fe240528c7df222d230bce613/> more work on docs
 - <csr-id-e4c06ce8e893779d2aad0883a1bb27d193bc5985/> update cargo tomls

### New Features

 - <csr-id-fd93ee89c19b085a04307ef30217170518defa8e/> upgrade syntax
 - <csr-id-2cf90b6903411e42f01a801f89037686194ee068/> pull children out of component definition
 - <csr-id-84fd0c616252bf29cd665782258530032b54d13a/> cleanuup
 - <csr-id-79503f15c5db04fa04575c8735941a2e3a75030b/> full html support
 - <csr-id-9726a065b0d4fb1ede5b53a2ddd58c855e51539f/> massage lifetimes
 - <csr-id-fac42339c272b0e430ebf4f31b6061a0635d3e19/> mutations
 - <csr-id-4a72b3140bd244da602deada1eeecded65ff5848/> amazingly awesome error handling
 - <csr-id-a2c7d17b0595769f60bc1c2bbf7cbe32cec37486/> mvoe away from compound context
 - <csr-id-7dfe89c9581f45a445f17f9fe4bb94e61f67e971/> wire up event delegator for webview
 - <csr-id-4091846934b4b3b2bc03d3ca8aaf7712aebd4e36/> add aria
 - <csr-id-e4cdb645aad800484b19ec35ba1f8bb9ccf71d12/> beaf up the use_state hook
 - <csr-id-7aec40d57e78ec13ff3a90ca8149521cbf1d9ff2/> enable arbitrary body in rsx! macro
 - <csr-id-22e659c2bd7797ca5a822180aca0cb5d950c5287/> namespaced attributes
   this commit adds namespaced attributes. This lets us support attribute groups, and thus, inline styles.
   
   This namespaced attribute stuff is only available for styles at the moment, though it theoretically could be enabled for any other attributes.
 - <csr-id-904b26f7111c3fc66400744ff6192e4b20bf6d74/> add edits back! and more webview support!
   This commit adds a new type - the DomEdit - for serializing the changes made by the diffing machine. The architecture of how DomEdits fit into the cooperative scheduling is still TBD but it will allow us to build change lists without applying them immediately. This is more performant  and allows us to only render parts of the page at a time.
   
   This commit also adds more infrastructure around webview. Dioxus can now run on the web, generate static pages, run in the desktop, and run on mobile, with a large part of thanks to webview.
 - <csr-id-b5e5ef171aa9f8986fb4ab04d793eb63f557c4ae/> two calculator examples
 - <csr-id-73047fe95678d50fcfd62a4ace7c6b406c5304e1/> props memoization is more powerful
   This commit solves the memoization , properly memoizing properties that don't have any generic parameters. This is a rough heuristic to prevent non-static lifetimes from creeping into props and breaking our minual lifetime management.
   
   Props that have a generic parameter are opted-out of the `partialeq` requirement and props *without* lifetimes must implement partialeq. We're going to leave manual disabling of memoization for future work.
 - <csr-id-cfa0927cdd40bc3dba22996018605dbad91d0391/> todomvc

### Bug Fixes

 - <csr-id-52c7154897111b570918127ffe3285bb1d5951a0/> really big bug around hooks
 - <csr-id-27d891934a70424b45e6278b7e2baaa2d1b78b35/> use annotation method from rust/58052 to fix closure lifetimes
 - <csr-id-ba9e1dbb8fa24048a6c9ccef8a8722688226a845/> messed up how lifetimes worked, need to render once per component
 - <csr-id-a33f7701fcf5f917fea8719253650b5ad92554fd/> tags
 - <csr-id-868f6739d2b2c5f2ace0c5240cff8008901e818c/> keyword length

