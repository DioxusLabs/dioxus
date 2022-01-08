# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.5 (2022-01-08)

### Documentation

 - <csr-id-e4c06ce8e893779d2aad0883a1bb27d193bc5985/> update cargo tomls
 - <csr-id-4de16c4779648e591b3869b5df31271ae603c812/> update local examples and docs to support new syntaxes
 - <csr-id-460783ad549818a85db634ed9c39ffce210b98ec/> lnks to projects

### New Features

 - <csr-id-8d685f40b7e0ef6521c60310d8687291e9b9c48a/> handle bool attrs properly
 - <csr-id-427b126bc17336d5d14d56eb7fddb8e07752495f/> add prevent default attribute and upgrade router
 - <csr-id-bbb6ee10de824f2e3259576ac01768640c884279/> make hydration more robust
 - <csr-id-d84fc0538670b2a3bda9ae41878896793b74e8ee/> plug in bubbling
 - <csr-id-420a30e5d432722e9da16311deb6aa60ea46b0cb/> overhaul examples and clean things up
 - <csr-id-a4f280d16399205c638033bf9beb858e478e98ff/> more API updates
 - <csr-id-b997b8ebbb82b5b9e9119bd2eb25335e2ed009d0/> enable children properly
 - <csr-id-19df1bda109aba03c40ff631263bcb7035004ca0/> bubbling
 - <csr-id-9d8c5ca5ab5784b3f17d7ee20a451ee68fd703d6/> it properly bubbles
 - <csr-id-fd93ee89c19b085a04307ef30217170518defa8e/> upgrade syntax
 - <csr-id-11757ddf61e1decb1bd1c2bb30455d0bd01a3e95/> fake bubbling
 - <csr-id-fda2ebc2a22965845e015384f39f34ce7cb3e428/> improve safety
 - <csr-id-9726a065b0d4fb1ede5b53a2ddd58c855e51539f/> massage lifetimes
 - <csr-id-efd0e9b5648c809057f339083ba9d454f810d483/> support desktop more completely
 - <csr-id-a2b0c50a343005c63c7032bcefb8323b78350bb9/> add update functionality to useref
 - <csr-id-5502429626023d0788cca352e94ac6ea67c2cb11/> desktop functioning well
 - <csr-id-4a4c7afca7e1beadd4b213214074fdb420eb0923/> shared state mechanisms
 - <csr-id-fac42339c272b0e430ebf4f31b6061a0635d3e19/> mutations
 - <csr-id-a2c7d17b0595769f60bc1c2bbf7cbe32cec37486/> mvoe away from compound context
 - <csr-id-f782e142118fb7acf1b88a0f3fbb03e4a5e3e91e/> omg what a dumb mistake
 - <csr-id-4a0068f09918adbc299150edcf777f342ced0dd3/> bless up, no more segfaults
 - <csr-id-7dfe89c9581f45a445f17f9fe4bb94e61f67e971/> wire up event delegator for webview

### Bug Fixes

 - <csr-id-21232285d9d84168d9003969ddd254fc22951e4b/> add exclusion list
 - <csr-id-bd341f5571580cdf5e495379b49ca988fd9211c3/> tests
 - <csr-id-3dc0e59876f5aba88ed26f1bbd692820f239d4b0/> readme and examples syntax
 - <csr-id-52c7154897111b570918127ffe3285bb1d5951a0/> really big bug around hooks
 - <csr-id-868f6739d2b2c5f2ace0c5240cff8008901e818c/> keyword length
 - <csr-id-a33f7701fcf5f917fea8719253650b5ad92554fd/> tags
 - <csr-id-601078f9cf78a58d7502a377676ac94f3cf037bf/> desktop and mobile
 - <csr-id-ba9e1dbb8fa24048a6c9ccef8a8722688226a845/> messed up how lifetimes worked, need to render once per component

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 91 commits contributed to the release over the course of 151 calendar days.
 - 78 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release dioxus-html v0.1.4, dioxus-desktop v0.1.5, dioxus-hooks v0.1.6, dioxus-mobile v0.0.3, dioxus-router v0.1.0, dioxus-ssr v0.1.2, dioxus-web v0.0.4, dioxus v0.1.7 ([`a36dab7`](https://github.comgit//DioxusLabs/dioxus/commit/a36dab7f45920acd8535a69b4aa3695f3bb92111))
    - Release dioxus-core v0.1.7, dioxus-core-macro v0.1.6, dioxus-html v0.1.4, dioxus-desktop v0.1.5, dioxus-hooks v0.1.6, dioxus-mobile v0.0.3, dioxus-router v0.1.0, dioxus-ssr v0.1.2, dioxus-web v0.0.4, dioxus v0.1.7 ([`40d1f85`](https://github.comgit//DioxusLabs/dioxus/commit/40d1f85d0c3e2c9fd23c08840cca9f459d4e4307))
    - add exclusion list ([`2123228`](https://github.comgit//DioxusLabs/dioxus/commit/21232285d9d84168d9003969ddd254fc22951e4b))
    - handle bool attrs properly ([`8d685f4`](https://github.comgit//DioxusLabs/dioxus/commit/8d685f40b7e0ef6521c60310d8687291e9b9c48a))
    - Merge pull request #89 from DioxusLabs/jk/simplify-example-run ([`8b6aa8b`](https://github.comgit//DioxusLabs/dioxus/commit/8b6aa8b880b6cb5c95e0c0743aad4e4e74388e05))
    - Merge pull request #74 from mrxiaozhuox/master ([`47056fd`](https://github.comgit//DioxusLabs/dioxus/commit/47056fda4577bcbdaa2a6f63d82eec876e5a5aee))
    - Merge pull request #80 from DioxusLabs/jk/router2dotoh ([`cdc2d8e`](https://github.comgit//DioxusLabs/dioxus/commit/cdc2d8ec6d123245c2ea5f6d10af02b6a6833994))
    - clear warnigns ([`175a6a1`](https://github.comgit//DioxusLabs/dioxus/commit/175a6a199c6738d8d0c7646ba0ec3fc4406c6535))
    - remove lag by forcing update ([`fd91158`](https://github.comgit//DioxusLabs/dioxus/commit/fd911584dcb2371070093e644d9b09255b573fc3))
    - add prevent default attribute and upgrade router ([`427b126`](https://github.comgit//DioxusLabs/dioxus/commit/427b126bc17336d5d14d56eb7fddb8e07752495f))
    - include desktop fixes ([`7cf15ee`](https://github.comgit//DioxusLabs/dioxus/commit/7cf15ee4e8dd5e57dd2a9af9d76f36cb7732e881))
    - make hydration more robust ([`bbb6ee1`](https://github.comgit//DioxusLabs/dioxus/commit/bbb6ee10de824f2e3259576ac01768640c884279))
    - Merge branch 'master' into jk/windows-desktop ([`be2d687`](https://github.comgit//DioxusLabs/dioxus/commit/be2d6876ab2fc35f3ae8ef863c2aaa8993196ac1))
    - try to fix pathing ([`ada24e7`](https://github.comgit//DioxusLabs/dioxus/commit/ada24e7c4eb96a9bf6ae2d1a9fdf1a77af0aa2c0))
    - bump all versions ([`4f92ba4`](https://github.comgit//DioxusLabs/dioxus/commit/4f92ba41602d706449c1bddabd49829873ee72eb))
    - tests ([`bd341f5`](https://github.comgit//DioxusLabs/dioxus/commit/bd341f5571580cdf5e495379b49ca988fd9211c3))
    - switch to log tracing ([`e2a6454`](https://github.comgit//DioxusLabs/dioxus/commit/e2a6454527cb81d24f7bd2a097beb644f34e3c2d))
    - bump desktop version ([`54103da`](https://github.comgit//DioxusLabs/dioxus/commit/54103da019655e1d70719ecef619a79edb5711db))
    - desktop ([`c1f8424`](https://github.comgit//DioxusLabs/dioxus/commit/c1f8424693b29cb8d7ead10d16e660674ca9264a))
    - desktop ([`be6fac9`](https://github.comgit//DioxusLabs/dioxus/commit/be6fac9f3dd24209b9a3b650636c2b807e335fef))
    - plug in bubbling ([`d84fc05`](https://github.comgit//DioxusLabs/dioxus/commit/d84fc0538670b2a3bda9ae41878896793b74e8ee))
    - overhaul examples and clean things up ([`420a30e`](https://github.comgit//DioxusLabs/dioxus/commit/420a30e5d432722e9da16311deb6aa60ea46b0cb))
    - remove runner on hook and then update docs ([`d156045`](https://github.comgit//DioxusLabs/dioxus/commit/d1560450bac55f9566e00e00ea405bd1c70b57e5))
    - polish some more things ([`1496102`](https://github.comgit//DioxusLabs/dioxus/commit/14961023f927b3a8bde83cfc7883aa8bfcca9e85))
    - more API updates ([`a4f280d`](https://github.comgit//DioxusLabs/dioxus/commit/a4f280d16399205c638033bf9beb858e478e98ff))
    - readme and examples syntax ([`3dc0e59`](https://github.comgit//DioxusLabs/dioxus/commit/3dc0e59876f5aba88ed26f1bbd692820f239d4b0))
    - rip out unsafe task engine ([`c7d001c`](https://github.comgit//DioxusLabs/dioxus/commit/c7d001cbb457929b9742ad96c4997cdcc695bb1a))
    - upgrade to new version of dioxus core. ([`cda759c`](https://github.comgit//DioxusLabs/dioxus/commit/cda759c659dfc4b1dde17e3896c35525005026df))
    - clean it up a bit ([`fa106be`](https://github.comgit//DioxusLabs/dioxus/commit/fa106be1f5a45fa5707e66542e52c9f09e8cea7a))
    - enable children properly ([`b997b8e`](https://github.comgit//DioxusLabs/dioxus/commit/b997b8ebbb82b5b9e9119bd2eb25335e2ed009d0))
    - miri stress tets ([`934de21`](https://github.comgit//DioxusLabs/dioxus/commit/934de21dd673b1b79904a3249998427f11428426))
    - rename fc to component ([`1e4a599`](https://github.comgit//DioxusLabs/dioxus/commit/1e4a599d14af85a2d1c29a442dd489f8dc8df321))
    - polish ([`8bf57dc`](https://github.comgit//DioxusLabs/dioxus/commit/8bf57dc21dfbcbae5b95650203b68d3f41227652))
    - prepare to change our fragment pattern. Add some more docs ([`2c3a046`](https://github.comgit//DioxusLabs/dioxus/commit/2c3a0464264fa11e8100df025d863931f9606cdb))
    - really big bug around hooks ([`52c7154`](https://github.comgit//DioxusLabs/dioxus/commit/52c7154897111b570918127ffe3285bb1d5951a0))
    - better desktop support ([`25a8411`](https://github.comgit//DioxusLabs/dioxus/commit/25a8411485e85bb7e3c8f20701d484529efe9a80))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.1 ([`2b92837`](https://github.comgit//DioxusLabs/dioxus/commit/2b928372fb1b74a4d4e220ff3d798bb7e52f79d2))
    - bubbling ([`19df1bd`](https://github.comgit//DioxusLabs/dioxus/commit/19df1bda109aba03c40ff631263bcb7035004ca0))
    - move examples around ([`1e6e5e6`](https://github.comgit//DioxusLabs/dioxus/commit/1e6e5e611b61571f272289adefc9cdd7d59c4399))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`0d480a4`](https://github.comgit//DioxusLabs/dioxus/commit/0d480a4c437d424f0eaff486e510a8fd3f3e6584))
    - updates to router ([`bab21a0`](https://github.comgit//DioxusLabs/dioxus/commit/bab21a0aa1cbf8e6bd95f823e49f53c082e8d6cc))
    - keyword length ([`868f673`](https://github.comgit//DioxusLabs/dioxus/commit/868f6739d2b2c5f2ace0c5240cff8008901e818c))
    - docs and router ([`a5f05d7`](https://github.comgit//DioxusLabs/dioxus/commit/a5f05d73acc0e47b05cff64a373482519414bc7c))
    - it properly bubbles ([`9d8c5ca`](https://github.comgit//DioxusLabs/dioxus/commit/9d8c5ca5ab5784b3f17d7ee20a451ee68fd703d6))
    - upgrade syntax ([`fd93ee8`](https://github.comgit//DioxusLabs/dioxus/commit/fd93ee89c19b085a04307ef30217170518defa8e))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`b32665d`](https://github.comgit//DioxusLabs/dioxus/commit/b32665d7212a5b9a3e21cb7af7abba63ae399fac))
    - fake bubbling ([`11757dd`](https://github.comgit//DioxusLabs/dioxus/commit/11757ddf61e1decb1bd1c2bb30455d0bd01a3e95))
    - tags ([`a33f770`](https://github.comgit//DioxusLabs/dioxus/commit/a33f7701fcf5f917fea8719253650b5ad92554fd))
    - Merge branch 'master' into jk/remove_node_safety ([`db00047`](https://github.comgit//DioxusLabs/dioxus/commit/db0004758c77331cc3b93ea8cf227c060028e12e))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`3a706ac`](https://github.comgit//DioxusLabs/dioxus/commit/3a706ac4168db137723bea90d7a0058190adfc3c))
    - Merge branch 'master' of https://github.com/jkelleyrtp/dioxus ([`60d6eb2`](https://github.comgit//DioxusLabs/dioxus/commit/60d6eb204a10633e5e52f91e855bd12c5cda40f2))
    - update cargo tomls ([`e4c06ce`](https://github.comgit//DioxusLabs/dioxus/commit/e4c06ce8e893779d2aad0883a1bb27d193bc5985))
    - desktop and mobile ([`601078f`](https://github.comgit//DioxusLabs/dioxus/commit/601078f9cf78a58d7502a377676ac94f3cf037bf))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`270dfc9`](https://github.comgit//DioxusLabs/dioxus/commit/270dfc9590b2354d083ea8da5cc0e1a1497d30e0))
    - slim down tokio ([`e86c1d8`](https://github.comgit//DioxusLabs/dioxus/commit/e86c1d8972dfa8717cd450513e90ff05b9af4776))
    - update local examples and docs to support new syntaxes ([`4de16c4`](https://github.comgit//DioxusLabs/dioxus/commit/4de16c4779648e591b3869b5df31271ae603c812))
    - docs ([`a42711a`](https://github.comgit//DioxusLabs/dioxus/commit/a42711a324215b87f607093a57b204be4154f30e))
    - improve safety ([`fda2ebc`](https://github.comgit//DioxusLabs/dioxus/commit/fda2ebc2a22965845e015384f39f34ce7cb3e428))
    - massage lifetimes ([`9726a06`](https://github.comgit//DioxusLabs/dioxus/commit/9726a065b0d4fb1ede5b53a2ddd58c855e51539f))
    - book documentation ([`16dbf4a`](https://github.comgit//DioxusLabs/dioxus/commit/16dbf4a6f84103857385fb4b142a718b0ce72118))
    - more changes to scheduler ([`059294a`](https://github.comgit//DioxusLabs/dioxus/commit/059294ab55e9e945c9aede1fd4b4faf39a7b9ea9))
    - messed up how lifetimes worked, need to render once per component ([`ba9e1db`](https://github.comgit//DioxusLabs/dioxus/commit/ba9e1dbb8fa24048a6c9ccef8a8722688226a845))
    - major cleanups to scheduler ([`2933e4b`](https://github.comgit//DioxusLabs/dioxus/commit/2933e4bc11b3074c2bde8d76ec55364fca841988))
    - move everything over to a stack dst ([`0e9d5fc`](https://github.comgit//DioxusLabs/dioxus/commit/0e9d5fc5306ab508d5af6999a4064f9b8b48460f))
    - support desktop more completely ([`efd0e9b`](https://github.comgit//DioxusLabs/dioxus/commit/efd0e9b5648c809057f339083ba9d454f810d483))
    - add update functionality to useref ([`a2b0c50`](https://github.comgit//DioxusLabs/dioxus/commit/a2b0c50a343005c63c7032bcefb8323b78350bb9))
    - lnks to projects ([`460783a`](https://github.comgit//DioxusLabs/dioxus/commit/460783ad549818a85db634ed9c39ffce210b98ec))
    - desktop functioning well ([`5502429`](https://github.comgit//DioxusLabs/dioxus/commit/5502429626023d0788cca352e94ac6ea67c2cb11))
    - more example images ([`2403990`](https://github.comgit//DioxusLabs/dioxus/commit/2403990ea362f1e066da5a877b123cbdfe3dada2))
    - overhaul event system ([`7a03c1d`](https://github.comgit//DioxusLabs/dioxus/commit/7a03c1d2b48590276b182465679387655fe08f3a))
    - threadsafe ([`82953f2`](https://github.comgit//DioxusLabs/dioxus/commit/82953f2ac37913f83a822333acd0c47e20777d31))
    - shared state mechanisms ([`4a4c7af`](https://github.comgit//DioxusLabs/dioxus/commit/4a4c7afca7e1beadd4b213214074fdb420eb0923))
    - clean up the web module ([`823adc0`](https://github.comgit//DioxusLabs/dioxus/commit/823adc0834b581327aee745c72ce8993f0bba5aa))
    - fix some event stuff for web and core ([`725b4a1`](https://github.comgit//DioxusLabs/dioxus/commit/725b4a1d7f5d629b1b0a163b65bfd93b2f8a151b))
    - mutations ([`fac4233`](https://github.comgit//DioxusLabs/dioxus/commit/fac42339c272b0e430ebf4f31b6061a0635d3e19))
    - add test_dom ([`a652090`](https://github.comgit//DioxusLabs/dioxus/commit/a652090dc5708db334fa7430fededb1bac207880))
    - bottom up dropping ([`f2334c1`](https://github.comgit//DioxusLabs/dioxus/commit/f2334c17be2612d926361686d7d40a57e3ffe9b9))
    - cleanup ([`1745a44`](https://github.comgit//DioxusLabs/dioxus/commit/1745a44d949b994b64ea1fb715cbe36963ae7027))
    - docs, html! macro, more ([`caf772c`](https://github.comgit//DioxusLabs/dioxus/commit/caf772cf249d2f56c8d0b0fa2737ad48e32c6e82))
    - cleanup workspace ([`8f0bb5d`](https://github.comgit//DioxusLabs/dioxus/commit/8f0bb5dc5bfa3e775af567c4b569622cdd932af1))
    - clean up warnings ([`b32e261`](https://github.comgit//DioxusLabs/dioxus/commit/b32e2611e37b17c2371ffb10cf1ac647f017d917))
    - web stuff ([`acad9ca`](https://github.comgit//DioxusLabs/dioxus/commit/acad9ca622748f96599dd02ad22aaeaae3621b76))
    - making progress on diffing and hydration ([`49856cc`](https://github.comgit//DioxusLabs/dioxus/commit/49856ccd6865f88d63765f26d27f7e945b554da0))
    - mvoe away from compound context ([`a2c7d17`](https://github.comgit//DioxusLabs/dioxus/commit/a2c7d17b0595769f60bc1c2bbf7cbe32cec37486))
    - omg what a dumb mistake ([`f782e14`](https://github.comgit//DioxusLabs/dioxus/commit/f782e142118fb7acf1b88a0f3fbb03e4a5e3e91e))
    - refactor ([`8b0eb87`](https://github.comgit//DioxusLabs/dioxus/commit/8b0eb87c72ea9d444dee99a8b05643f19fea2634))
    - bless up, no more segfaults ([`4a0068f`](https://github.comgit//DioxusLabs/dioxus/commit/4a0068f09918adbc299150edcf777f342ced0dd3))
    - wire up event delegator for webview ([`7dfe89c`](https://github.comgit//DioxusLabs/dioxus/commit/7dfe89c9581f45a445f17f9fe4bb94e61f67e971))
    - solve some issues regarding listeners ([`dfaf5ad`](https://github.comgit//DioxusLabs/dioxus/commit/dfaf5adee164f44a679ab21d730caaab3610e01f))
    - more overhaul on virtualevents ([`41cc429`](https://github.comgit//DioxusLabs/dioxus/commit/41cc42919d42453f8f2560aa852211364af4ad3d))
    - groundwork for noderefs ([`c1afeba`](https://github.comgit//DioxusLabs/dioxus/commit/c1afeba1efb1a063705466a14648beee08cacb86))
</details>

## v0.0.0 (2021-12-15)

### Documentation

 - <csr-id-4de16c4779648e591b3869b5df31271ae603c812/> update local examples and docs to support new syntaxes
 - <csr-id-460783ad549818a85db634ed9c39ffce210b98ec/> lnks to projects
 - <csr-id-e4c06ce8e893779d2aad0883a1bb27d193bc5985/> update cargo tomls

### New Features

 - <csr-id-19df1bda109aba03c40ff631263bcb7035004ca0/> bubbling
 - <csr-id-9d8c5ca5ab5784b3f17d7ee20a451ee68fd703d6/> it properly bubbles
 - <csr-id-fd93ee89c19b085a04307ef30217170518defa8e/> upgrade syntax
 - <csr-id-11757ddf61e1decb1bd1c2bb30455d0bd01a3e95/> fake bubbling
 - <csr-id-fda2ebc2a22965845e015384f39f34ce7cb3e428/> improve safety
 - <csr-id-9726a065b0d4fb1ede5b53a2ddd58c855e51539f/> massage lifetimes
 - <csr-id-efd0e9b5648c809057f339083ba9d454f810d483/> support desktop more completely
 - <csr-id-a2b0c50a343005c63c7032bcefb8323b78350bb9/> add update functionality to useref
 - <csr-id-5502429626023d0788cca352e94ac6ea67c2cb11/> desktop functioning well
 - <csr-id-4a4c7afca7e1beadd4b213214074fdb420eb0923/> shared state mechanisms
 - <csr-id-fac42339c272b0e430ebf4f31b6061a0635d3e19/> mutations
 - <csr-id-a2c7d17b0595769f60bc1c2bbf7cbe32cec37486/> mvoe away from compound context
 - <csr-id-f782e142118fb7acf1b88a0f3fbb03e4a5e3e91e/> omg what a dumb mistake
 - <csr-id-4a0068f09918adbc299150edcf777f342ced0dd3/> bless up, no more segfaults
 - <csr-id-7dfe89c9581f45a445f17f9fe4bb94e61f67e971/> wire up event delegator for webview

### Bug Fixes

 - <csr-id-52c7154897111b570918127ffe3285bb1d5951a0/> really big bug around hooks
 - <csr-id-601078f9cf78a58d7502a377676ac94f3cf037bf/> desktop and mobile
 - <csr-id-ba9e1dbb8fa24048a6c9ccef8a8722688226a845/> messed up how lifetimes worked, need to render once per component
 - <csr-id-a33f7701fcf5f917fea8719253650b5ad92554fd/> tags
 - <csr-id-868f6739d2b2c5f2ace0c5240cff8008901e818c/> keyword length

