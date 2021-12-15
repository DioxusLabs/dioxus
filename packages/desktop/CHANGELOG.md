# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 55 commits contributed to the release over the course of 128 calendar days.
 - 51 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - tags ([`a33f770`](https://github.comgit//DioxusLabs/dioxus/commit/a33f7701fcf5f917fea8719253650b5ad92554fd))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`3a706ac`](https://github.comgit//DioxusLabs/dioxus/commit/3a706ac4168db137723bea90d7a0058190adfc3c))
    - update cargo tomls ([`e4c06ce`](https://github.comgit//DioxusLabs/dioxus/commit/e4c06ce8e893779d2aad0883a1bb27d193bc5985))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`270dfc9`](https://github.comgit//DioxusLabs/dioxus/commit/270dfc9590b2354d083ea8da5cc0e1a1497d30e0))
    - update local examples and docs to support new syntaxes ([`4de16c4`](https://github.comgit//DioxusLabs/dioxus/commit/4de16c4779648e591b3869b5df31271ae603c812))
    - rename fc to component ([`1e4a599`](https://github.comgit//DioxusLabs/dioxus/commit/1e4a599d14af85a2d1c29a442dd489f8dc8df321))
    - polish ([`8bf57dc`](https://github.comgit//DioxusLabs/dioxus/commit/8bf57dc21dfbcbae5b95650203b68d3f41227652))
    - really big bug around hooks ([`52c7154`](https://github.comgit//DioxusLabs/dioxus/commit/52c7154897111b570918127ffe3285bb1d5951a0))
    - better desktop support ([`25a8411`](https://github.comgit//DioxusLabs/dioxus/commit/25a8411485e85bb7e3c8f20701d484529efe9a80))
    - bubbling ([`19df1bd`](https://github.comgit//DioxusLabs/dioxus/commit/19df1bda109aba03c40ff631263bcb7035004ca0))
    - move examples around ([`1e6e5e6`](https://github.comgit//DioxusLabs/dioxus/commit/1e6e5e611b61571f272289adefc9cdd7d59c4399))
    - updates to router ([`bab21a0`](https://github.comgit//DioxusLabs/dioxus/commit/bab21a0aa1cbf8e6bd95f823e49f53c082e8d6cc))
    - docs and router ([`a5f05d7`](https://github.comgit//DioxusLabs/dioxus/commit/a5f05d73acc0e47b05cff64a373482519414bc7c))
    - it properly bubbles ([`9d8c5ca`](https://github.comgit//DioxusLabs/dioxus/commit/9d8c5ca5ab5784b3f17d7ee20a451ee68fd703d6))
    - upgrade syntax ([`fd93ee8`](https://github.comgit//DioxusLabs/dioxus/commit/fd93ee89c19b085a04307ef30217170518defa8e))
    - fake bubbling ([`11757dd`](https://github.comgit//DioxusLabs/dioxus/commit/11757ddf61e1decb1bd1c2bb30455d0bd01a3e95))
    - Merge branch 'master' into jk/remove_node_safety ([`db00047`](https://github.comgit//DioxusLabs/dioxus/commit/db0004758c77331cc3b93ea8cf227c060028e12e))
    - Merge branch 'master' of https://github.com/jkelleyrtp/dioxus ([`60d6eb2`](https://github.comgit//DioxusLabs/dioxus/commit/60d6eb204a10633e5e52f91e855bd12c5cda40f2))
    - desktop and mobile ([`601078f`](https://github.comgit//DioxusLabs/dioxus/commit/601078f9cf78a58d7502a377676ac94f3cf037bf))
    - slim down tokio ([`e86c1d8`](https://github.comgit//DioxusLabs/dioxus/commit/e86c1d8972dfa8717cd450513e90ff05b9af4776))
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

