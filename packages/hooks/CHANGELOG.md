# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.3 (2021-12-15)

### Documentation

 - <csr-id-4de16c4779648e591b3869b5df31271ae603c812/> update local examples and docs to support new syntaxes
 - <csr-id-583fdfa5618e11d660985b97e570d4503be2ff49/> big updates to the reference
 - <csr-id-70cd46dbb2a689ae2d512e142b8aee9c80798430/> move around examples
 - <csr-id-69f5cc3802af136729bc73e5c3d209270d41b184/> move into a fromjs tutorial
 - <csr-id-e4c06ce8e893779d2aad0883a1bb27d193bc5985/> update cargo tomls

### New Features

 - <csr-id-8acdd2ea830b995b608d8bac2ef527db8d40e662/> it compiles once more
 - <csr-id-fda2ebc2a22965845e015384f39f34ce7cb3e428/> improve safety
 - <csr-id-9726a065b0d4fb1ede5b53a2ddd58c855e51539f/> massage lifetimes
 - <csr-id-a2b0c50a343005c63c7032bcefb8323b78350bb9/> add update functionality to useref
 - <csr-id-4a4c7afca7e1beadd4b213214074fdb420eb0923/> shared state mechanisms
 - <csr-id-a2c7d17b0595769f60bc1c2bbf7cbe32cec37486/> mvoe away from compound context
 - <csr-id-f782e142118fb7acf1b88a0f3fbb03e4a5e3e91e/> omg what a dumb mistake
 - <csr-id-de9f61bcf48c0d6e35e46c337b72a713c9f9f7d2/> more suspended nodes!
 - <csr-id-7dfe89c9581f45a445f17f9fe4bb94e61f67e971/> wire up event delegator for webview
 - <csr-id-3a57b942624afb8aa6650aeee05466c3c9ce967e/> task system works
   but I broke the other things :(
 - <csr-id-9abb0470b7869019d539a2fc21da3872348ae38b/> static node infrastructure and ssr changes

### Bug Fixes

 - <csr-id-52c7154897111b570918127ffe3285bb1d5951a0/> really big bug around hooks
 - <csr-id-601078f9cf78a58d7502a377676ac94f3cf037bf/> desktop and mobile
 - <csr-id-478255f40d4de1d2e3f3cc9b6d758b30ff394b39/> all the bugs!

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 60 commits contributed to the release over the course of 335 calendar days.
 - 58 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - update cargo tomls ([`e4c06ce`](https://github.comgit//DioxusLabs/dioxus/commit/e4c06ce8e893779d2aad0883a1bb27d193bc5985))
    - update local examples and docs to support new syntaxes ([`4de16c4`](https://github.comgit//DioxusLabs/dioxus/commit/4de16c4779648e591b3869b5df31271ae603c812))
    - docs ([`3ddf395`](https://github.comgit//DioxusLabs/dioxus/commit/3ddf395772aa24d04cf4e35bbb3a0027c8f1c7dd))
    - remove portals completely ([`2fd56e7`](https://github.comgit//DioxusLabs/dioxus/commit/2fd56e76192bc70d5503bfcd6b4127d383dd082c))
    - go back to noisy lifetime solution ([`8daf7a6`](https://github.comgit//DioxusLabs/dioxus/commit/8daf7a6ed86df72522b089aa2647eea7bee0f3b6))
    - clean up the core crate ([`e6c6bbd`](https://github.comgit//DioxusLabs/dioxus/commit/e6c6bbdc1ec6a8c251b78c05ca104f006b6fad26))
    - make warnings go away ([`0545d27`](https://github.comgit//DioxusLabs/dioxus/commit/0545d271828418f3ab0df2e9abf66ccafb30707d))
    - rename fc to component ([`1e4a599`](https://github.comgit//DioxusLabs/dioxus/commit/1e4a599d14af85a2d1c29a442dd489f8dc8df321))
    - docs ([`8814977`](https://github.comgit//DioxusLabs/dioxus/commit/8814977eeebe06748a3b9677a8070e42a037ebd7))
    - update hooks ([`597a045`](https://github.comgit//DioxusLabs/dioxus/commit/597a0456f59872bd5dc60d382acdec76a98b1db2))
    - really big bug around hooks ([`52c7154`](https://github.comgit//DioxusLabs/dioxus/commit/52c7154897111b570918127ffe3285bb1d5951a0))
    - better desktop support ([`25a8411`](https://github.comgit//DioxusLabs/dioxus/commit/25a8411485e85bb7e3c8f20701d484529efe9a80))
    - it compiles once more ([`8acdd2e`](https://github.comgit//DioxusLabs/dioxus/commit/8acdd2ea830b995b608d8bac2ef527db8d40e662))
    - move examples around ([`1e6e5e6`](https://github.comgit//DioxusLabs/dioxus/commit/1e6e5e611b61571f272289adefc9cdd7d59c4399))
    - docs and router ([`a5f05d7`](https://github.comgit//DioxusLabs/dioxus/commit/a5f05d73acc0e47b05cff64a373482519414bc7c))
    - Merge branch 'master' into jk/remove_node_safety ([`db00047`](https://github.comgit//DioxusLabs/dioxus/commit/db0004758c77331cc3b93ea8cf227c060028e12e))
    - Merge branch 'master' of https://github.com/jkelleyrtp/dioxus ([`60d6eb2`](https://github.comgit//DioxusLabs/dioxus/commit/60d6eb204a10633e5e52f91e855bd12c5cda40f2))
    - bubbling in progress ([`a21020e`](https://github.comgit//DioxusLabs/dioxus/commit/a21020ea575e467ba0d608737269fe1b0792dba7))
    - desktop and mobile ([`601078f`](https://github.comgit//DioxusLabs/dioxus/commit/601078f9cf78a58d7502a377676ac94f3cf037bf))
    - improve safety ([`fda2ebc`](https://github.comgit//DioxusLabs/dioxus/commit/fda2ebc2a22965845e015384f39f34ce7cb3e428))
    - massage lifetimes ([`9726a06`](https://github.comgit//DioxusLabs/dioxus/commit/9726a065b0d4fb1ede5b53a2ddd58c855e51539f))
    - major cleanups to scheduler ([`2933e4b`](https://github.comgit//DioxusLabs/dioxus/commit/2933e4bc11b3074c2bde8d76ec55364fca841988))
    - add update functionality to useref ([`a2b0c50`](https://github.comgit//DioxusLabs/dioxus/commit/a2b0c50a343005c63c7032bcefb8323b78350bb9))
    - all the bugs! ([`478255f`](https://github.comgit//DioxusLabs/dioxus/commit/478255f40d4de1d2e3f3cc9b6d758b30ff394b39))
    - shared state mechanisms ([`4a4c7af`](https://github.comgit//DioxusLabs/dioxus/commit/4a4c7afca7e1beadd4b213214074fdb420eb0923))
    - fix web list issue ([`da4423c`](https://github.comgit//DioxusLabs/dioxus/commit/da4423c141f1f376df5f3f2580e5284831744a7e))
    - remove wildcard ([`ba8ced5`](https://github.comgit//DioxusLabs/dioxus/commit/ba8ced573caea6f55d47804c327d6a279d4733a6))
    - clean up the web module ([`823adc0`](https://github.comgit//DioxusLabs/dioxus/commit/823adc0834b581327aee745c72ce8993f0bba5aa))
    - examples ([`1a2f91e`](https://github.comgit//DioxusLabs/dioxus/commit/1a2f91ed91c13dae553ecde585462ab261b1b95d))
    - performance looks good, needs more testing ([`4b6ca05`](https://github.comgit//DioxusLabs/dioxus/commit/4b6ca05f2c3ad647842c858967da9c87f1915825))
    - cleanup ([`1745a44`](https://github.comgit//DioxusLabs/dioxus/commit/1745a44d949b994b64ea1fb715cbe36963ae7027))
    - fill out the snippets ([`6051b0e`](https://github.comgit//DioxusLabs/dioxus/commit/6051b0ec86927704451f4ce6cdf8f988e59702ae))
    - big updates to the reference ([`583fdfa`](https://github.comgit//DioxusLabs/dioxus/commit/583fdfa5618e11d660985b97e570d4503be2ff49))
    - docs, html! macro, more ([`caf772c`](https://github.comgit//DioxusLabs/dioxus/commit/caf772cf249d2f56c8d0b0fa2737ad48e32c6e82))
    - mvoe away from compound context ([`a2c7d17`](https://github.comgit//DioxusLabs/dioxus/commit/a2c7d17b0595769f60bc1c2bbf7cbe32cec37486))
    - examples ([`f1cff84`](https://github.comgit//DioxusLabs/dioxus/commit/f1cff845ce11231cb4b2dd4857f9ca9f0265b925))
    - omg what a dumb mistake ([`f782e14`](https://github.comgit//DioxusLabs/dioxus/commit/f782e142118fb7acf1b88a0f3fbb03e4a5e3e91e))
    - more suspended nodes! ([`de9f61b`](https://github.comgit//DioxusLabs/dioxus/commit/de9f61bcf48c0d6e35e46c337b72a713c9f9f7d2))
    - wire up event delegator for webview ([`7dfe89c`](https://github.comgit//DioxusLabs/dioxus/commit/7dfe89c9581f45a445f17f9fe4bb94e61f67e971))
    - task system works ([`3a57b94`](https://github.comgit//DioxusLabs/dioxus/commit/3a57b942624afb8aa6650aeee05466c3c9ce967e))
    - move examples around ([`304259d`](https://github.comgit//DioxusLabs/dioxus/commit/304259d8186d1d34224a74c95f4fd7d14126b499))
    - ssr + tide ([`269e81b`](https://github.comgit//DioxusLabs/dioxus/commit/269e81b0fdb32ae0706160cd278cf3a1b731387b))
    - static node infrastructure and ssr changes ([`9abb047`](https://github.comgit//DioxusLabs/dioxus/commit/9abb0470b7869019d539a2fc21da3872348ae38b))
    - more refactor for async ([`975fa56`](https://github.comgit//DioxusLabs/dioxus/commit/975fa566f9809f8fa2bb0bdb07fbfc7f855dcaeb))
    - some project refactor ([`8cfc437`](https://github.comgit//DioxusLabs/dioxus/commit/8cfc437bfe4110d7f984428f01df90bdf8f8d9ec))
    - move some examples around ([`98a0933`](https://github.comgit//DioxusLabs/dioxus/commit/98a09339fd3190799ea4dd316908f0a53fdf2413))
    - rename ctx to cx ([`81382e7`](https://github.comgit//DioxusLabs/dioxus/commit/81382e7044fb3dba61d4abb1e6086b7b29143116))
    - move around examples ([`70cd46d`](https://github.comgit//DioxusLabs/dioxus/commit/70cd46dbb2a689ae2d512e142b8aee9c80798430))
    - pre vnodes instead of vnode ([`fe6938c`](https://github.comgit//DioxusLabs/dioxus/commit/fe6938ceb3dba0796ae8bab52ae41248dc0d3650))
    - move into a fromjs tutorial ([`69f5cc3`](https://github.comgit//DioxusLabs/dioxus/commit/69f5cc3802af136729bc73e5c3d209270d41b184))
    - massive changes to definition of components ([`508c560`](https://github.comgit//DioxusLabs/dioxus/commit/508c560320d78730fa058156421523ffa5695d9d))
    - major overhaul to diffing ([`9810fee`](https://github.comgit//DioxusLabs/dioxus/commit/9810feebf57f93114e3d7faf6de053ac192593a9))
    - cargo fix to clean up things ([`78d093a`](https://github.comgit//DioxusLabs/dioxus/commit/78d093a9454386397a991bd01e603e4ad554521f))
    - overall API updates ([`f47651b`](https://github.comgit//DioxusLabs/dioxus/commit/f47651b32afb385297ddae00616f4308083a36e6))
    - implememt nodes better ([`edbb33b`](https://github.comgit//DioxusLabs/dioxus/commit/edbb33b2ee3692cac155b9e206cd7c2d6382eb9d))
    - comment out examples and move lifetime in FC type ([`62d4ad5`](https://github.comgit//DioxusLabs/dioxus/commit/62d4ad58787185032100a2d25e79b70f6ec97a3c))
    - updates to docs, extension ([`a2406b3`](https://github.comgit//DioxusLabs/dioxus/commit/a2406b33d6c11174f95bd003a59650b86f14159f))
    - add webview example ([`65d0d61`](https://github.comgit//DioxusLabs/dioxus/commit/65d0d611ea9a47d305151d65769b52ec22559959))
    - add hooks ([`c1b990b`](https://github.comgit//DioxusLabs/dioxus/commit/c1b990b27c34e5d6b95ec78e07394b3806b75dc1))
    - docs, code frm percy ([`2b9c8d0`](https://github.comgit//DioxusLabs/dioxus/commit/2b9c8d09d926ff6b5ad8a7e7b7b0b6f93bb8eb36))
</details>

