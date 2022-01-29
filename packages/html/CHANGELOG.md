# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Bug Fixes

 - <csr-id-43e78d56f72e23ada939f5688a6bd2fdd9b8292d/> rustfmt

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 19 calendar days.
 - 1 commit where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - rustfmt ([`43e78d5`](https://github.comgit//DioxusLabs/dioxus/commit/43e78d56f72e23ada939f5688a6bd2fdd9b8292d))
    - Add gap and row_gap to style_trait_methods ([`5f4a724`](https://github.comgit//DioxusLabs/dioxus/commit/5f4a72446e63441628e18bb19fd9b2d9f65f9d52))
    - Merge pull request #111 from DioxusLabs/jk/props-attrs ([`0369fe7`](https://github.comgit//DioxusLabs/dioxus/commit/0369fe72fb247409da300a54ef11ba9155d0efb3))
    - Fix various typos and grammar nits ([`9e4ec43`](https://github.comgit//DioxusLabs/dioxus/commit/9e4ec43b1e78d355c56a38e4c092170b2b01b20d))
    - Merge pull request #113 from DioxusLabs/jk/desktop-cursor-jump ([`20a2940`](https://github.comgit//DioxusLabs/dioxus/commit/20a29409b22510b001fdbee349724adb7b44d401))
    - feat(events:focus): add missing `onfocusin` event ([`007d06d`](https://github.comgit//DioxusLabs/dioxus/commit/007d06d602f1adfaa51c87ec89b2afe90d8cdef9))
    - feat(example:todomvc): add editing support ([`9849f68`](https://github.comgit//DioxusLabs/dioxus/commit/9849f68f257200fac511c048bfb1a076243b86d3))
</details>

## v0.1.4 (2022-01-08)

### Documentation

 - <csr-id-e4c06ce8e893779d2aad0883a1bb27d193bc5985/> update cargo tomls
 - <csr-id-285b33dd38c88e2be797eb7f19c34a0906f16b13/> update readme
 - <csr-id-583fdfa5618e11d660985b97e570d4503be2ff49/> big updates to the reference

### New Features

 - <csr-id-9dff700c220dd9e0da2ee028900e82fd24f9d0dd/> enable prevent_default everywhere
 - <csr-id-06276edd0d4f1f6f6b0a3bf7a467931413ab33c3/> eanble bubbling
 - <csr-id-d84fc0538670b2a3bda9ae41878896793b74e8ee/> plug in bubbling
 - <csr-id-11757ddf61e1decb1bd1c2bb30455d0bd01a3e95/> fake bubbling
 - <csr-id-f2234068ba7cd915a00a81e41660d7d6ee1177cc/> events bubble now
 - <csr-id-cfc24f5451cd2d1e9dcd5f1589ee50f705404110/> support innerhtml
 - <csr-id-4a4c7afca7e1beadd4b213214074fdb420eb0923/> shared state mechanisms
 - <csr-id-718fa14b45df38b40b0c0dff7bdc923cba57026b/> a cute crm
 - <csr-id-3bedcb93cacec5bdf134adc38ff02eadbf96c1c6/> svgs working in webview
 - <csr-id-de9f61bcf48c0d6e35e46c337b72a713c9f9f7d2/> more suspended nodes!
 - <csr-id-4091846934b4b3b2bc03d3ca8aaf7712aebd4e36/> add aria
 - <csr-id-c79d9ae674e235c8e9c2c069d24902122b9c7464/> buff up html allowed attributes
 - <csr-id-e4cdb645aad800484b19ec35ba1f8bb9ccf71d12/> beaf up the use_state hook
 - <csr-id-7aec40d57e78ec13ff3a90ca8149521cbf1d9ff2/> enable arbitrary body in rsx! macro

### Bug Fixes

 - <csr-id-c439b0ac7e09f70a04262b7c29938d8c52197b76/> component pass thru events
 - <csr-id-75fa7b4aa672a8a10afcd11016a1b80e0e6f0f02/> make tests pass
 - <csr-id-868f6739d2b2c5f2ace0c5240cff8008901e818c/> keyword length
 - <csr-id-a33f7701fcf5f917fea8719253650b5ad92554fd/> tags
 - <csr-id-601078f9cf78a58d7502a377676ac94f3cf037bf/> desktop and mobile

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 61 commits contributed to the release over the course of 184 calendar days.
 - 52 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release dioxus-html v0.1.4, dioxus-desktop v0.1.5, dioxus-hooks v0.1.6, dioxus-mobile v0.0.3, dioxus-router v0.1.0, dioxus-ssr v0.1.2, dioxus-web v0.0.4, dioxus v0.1.7 ([`a36dab7`](https://github.comgit//DioxusLabs/dioxus/commit/a36dab7f45920acd8535a69b4aa3695f3bb92111))
    - enable prevent_default everywhere ([`9dff700`](https://github.comgit//DioxusLabs/dioxus/commit/9dff700c220dd9e0da2ee028900e82fd24f9d0dd))
    - Release dioxus-core v0.1.7, dioxus-core-macro v0.1.6, dioxus-html v0.1.4, dioxus-desktop v0.1.5, dioxus-hooks v0.1.6, dioxus-mobile v0.0.3, dioxus-router v0.1.0, dioxus-ssr v0.1.2, dioxus-web v0.0.4, dioxus v0.1.7 ([`40d1f85`](https://github.comgit//DioxusLabs/dioxus/commit/40d1f85d0c3e2c9fd23c08840cca9f459d4e4307))
    - component pass thru events ([`c439b0a`](https://github.comgit//DioxusLabs/dioxus/commit/c439b0ac7e09f70a04262b7c29938d8c52197b76))
    - memoize dom in the prescence of identical components ([`cb2782b`](https://github.comgit//DioxusLabs/dioxus/commit/cb2782b4bb34cdaadfff590bfee930ae3ac6536c))
    - new versions of everything ([`4ea5c99`](https://github.comgit//DioxusLabs/dioxus/commit/4ea5c990d72b1645724ab0a88ffea2baf28e2835))
    - bump all versions ([`4f92ba4`](https://github.comgit//DioxusLabs/dioxus/commit/4f92ba41602d706449c1bddabd49829873ee72eb))
    - update core, core-macro, and html ([`f9b9bb9`](https://github.comgit//DioxusLabs/dioxus/commit/f9b9bb9c0c2c55f55d2d6860e3d2d986debd6412))
    - eanble bubbling ([`06276ed`](https://github.comgit//DioxusLabs/dioxus/commit/06276edd0d4f1f6f6b0a3bf7a467931413ab33c3))
    - plug in bubbling ([`d84fc05`](https://github.comgit//DioxusLabs/dioxus/commit/d84fc0538670b2a3bda9ae41878896793b74e8ee))
    - make tests pass ([`75fa7b4`](https://github.comgit//DioxusLabs/dioxus/commit/75fa7b4aa672a8a10afcd11016a1b80e0e6f0f02))
    - remove runner on hook and then update docs ([`d156045`](https://github.comgit//DioxusLabs/dioxus/commit/d1560450bac55f9566e00e00ea405bd1c70b57e5))
    - bump html crate ([`18f1fa4`](https://github.comgit//DioxusLabs/dioxus/commit/18f1fa463780885ec7281bf4a2256d3fecc146ea))
    - add more svg elements ([`8f9a328`](https://github.comgit//DioxusLabs/dioxus/commit/8f9a3281e79273ae9e366c2ce1c28e068112371d))
    - add more svg elements, update readme ([`ad4a0eb`](https://github.comgit//DioxusLabs/dioxus/commit/ad4a0eb3191cefcad3c570517f15f5c0fd7e8687))
    - polish some more things ([`1496102`](https://github.comgit//DioxusLabs/dioxus/commit/14961023f927b3a8bde83cfc7883aa8bfcca9e85))
    - upgrade hooks ([`b3ac2ee`](https://github.comgit//DioxusLabs/dioxus/commit/b3ac2ee3f76549cd1c7b6f9eee7e3382b07d873c))
    - fix documentation page ([`42c6d17`](https://github.comgit//DioxusLabs/dioxus/commit/42c6d1772b71b061aed1987819df1eaf5e951bde))
    - rename fc to component ([`1e4a599`](https://github.comgit//DioxusLabs/dioxus/commit/1e4a599d14af85a2d1c29a442dd489f8dc8df321))
    - polish ([`8bf57dc`](https://github.comgit//DioxusLabs/dioxus/commit/8bf57dc21dfbcbae5b95650203b68d3f41227652))
    - prepare to change our fragment pattern. Add some more docs ([`2c3a046`](https://github.comgit//DioxusLabs/dioxus/commit/2c3a0464264fa11e8100df025d863931f9606cdb))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.1 ([`2b92837`](https://github.comgit//DioxusLabs/dioxus/commit/2b928372fb1b74a4d4e220ff3d798bb7e52f79d2))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`0d480a4`](https://github.comgit//DioxusLabs/dioxus/commit/0d480a4c437d424f0eaff486e510a8fd3f3e6584))
    - keyword length ([`868f673`](https://github.comgit//DioxusLabs/dioxus/commit/868f6739d2b2c5f2ace0c5240cff8008901e818c))
    - docs and router ([`a5f05d7`](https://github.comgit//DioxusLabs/dioxus/commit/a5f05d73acc0e47b05cff64a373482519414bc7c))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`b32665d`](https://github.comgit//DioxusLabs/dioxus/commit/b32665d7212a5b9a3e21cb7af7abba63ae399fac))
    - fake bubbling ([`11757dd`](https://github.comgit//DioxusLabs/dioxus/commit/11757ddf61e1decb1bd1c2bb30455d0bd01a3e95))
    - tags ([`a33f770`](https://github.comgit//DioxusLabs/dioxus/commit/a33f7701fcf5f917fea8719253650b5ad92554fd))
    - events bubble now ([`f223406`](https://github.comgit//DioxusLabs/dioxus/commit/f2234068ba7cd915a00a81e41660d7d6ee1177cc))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`3a706ac`](https://github.comgit//DioxusLabs/dioxus/commit/3a706ac4168db137723bea90d7a0058190adfc3c))
    - Merge branch 'master' of https://github.com/jkelleyrtp/dioxus ([`60d6eb2`](https://github.comgit//DioxusLabs/dioxus/commit/60d6eb204a10633e5e52f91e855bd12c5cda40f2))
    - update cargo tomls ([`e4c06ce`](https://github.comgit//DioxusLabs/dioxus/commit/e4c06ce8e893779d2aad0883a1bb27d193bc5985))
    - desktop and mobile ([`601078f`](https://github.comgit//DioxusLabs/dioxus/commit/601078f9cf78a58d7502a377676ac94f3cf037bf))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`270dfc9`](https://github.comgit//DioxusLabs/dioxus/commit/270dfc9590b2354d083ea8da5cc0e1a1497d30e0))
    - bump versions ([`0846d93`](https://github.comgit//DioxusLabs/dioxus/commit/0846d93d41c27464ca271757c6f24a2cef8fb997))
    - support innerhtml ([`cfc24f5`](https://github.comgit//DioxusLabs/dioxus/commit/cfc24f5451cd2d1e9dcd5f1589ee50f705404110))
    - major cleanups to scheduler ([`2933e4b`](https://github.comgit//DioxusLabs/dioxus/commit/2933e4bc11b3074c2bde8d76ec55364fca841988))
    - more raw ptrs ([`95bd17e`](https://github.comgit//DioxusLabs/dioxus/commit/95bd17e38fc936dcc9383d0ba8beac5ed64b41eb))
    - update readme ([`285b33d`](https://github.comgit//DioxusLabs/dioxus/commit/285b33dd38c88e2be797eb7f19c34a0906f16b13))
    - shared state mechanisms ([`4a4c7af`](https://github.comgit//DioxusLabs/dioxus/commit/4a4c7afca7e1beadd4b213214074fdb420eb0923))
    - fix web list issue ([`da4423c`](https://github.comgit//DioxusLabs/dioxus/commit/da4423c141f1f376df5f3f2580e5284831744a7e))
    - html package ([`740cbd1`](https://github.comgit//DioxusLabs/dioxus/commit/740cbd1f9d3e7055822af0be5cc07cc8140eb435))
    - and publish ([`51e2005`](https://github.comgit//DioxusLabs/dioxus/commit/51e20052d9f711a06232675eddc6aace492ad287))
    - clean up the web module ([`823adc0`](https://github.comgit//DioxusLabs/dioxus/commit/823adc0834b581327aee745c72ce8993f0bba5aa))
    - a cute crm ([`718fa14`](https://github.comgit//DioxusLabs/dioxus/commit/718fa14b45df38b40b0c0dff7bdc923cba57026b))
    - examples ([`1a2f91e`](https://github.comgit//DioxusLabs/dioxus/commit/1a2f91ed91c13dae553ecde585462ab261b1b95d))
    - some ideas ([`05c909f`](https://github.comgit//DioxusLabs/dioxus/commit/05c909f320765aec1bf4c1c55ca59ffd5525a2c7))
    - big updates to the reference ([`583fdfa`](https://github.comgit//DioxusLabs/dioxus/commit/583fdfa5618e11d660985b97e570d4503be2ff49))
    - cleanup workspace ([`8f0bb5d`](https://github.comgit//DioxusLabs/dioxus/commit/8f0bb5dc5bfa3e775af567c4b569622cdd932af1))
    - svgs working in webview ([`3bedcb9`](https://github.comgit//DioxusLabs/dioxus/commit/3bedcb93cacec5bdf134adc38ff02eadbf96c1c6))
    - more doc ([`adf202e`](https://github.comgit//DioxusLabs/dioxus/commit/adf202eab9201c69ec455e621c755500329815fe))
    - more suspended nodes! ([`de9f61b`](https://github.comgit//DioxusLabs/dioxus/commit/de9f61bcf48c0d6e35e46c337b72a713c9f9f7d2))
    - solve some issues regarding listeners ([`dfaf5ad`](https://github.comgit//DioxusLabs/dioxus/commit/dfaf5adee164f44a679ab21d730caaab3610e01f))
    - wip ([`996247a`](https://github.comgit//DioxusLabs/dioxus/commit/996247a1644d91ebb00e2b2188d21cacdc48257b))
    - add aria ([`4091846`](https://github.comgit//DioxusLabs/dioxus/commit/4091846934b4b3b2bc03d3ca8aaf7712aebd4e36))
    - more examples ([`56e7eb8`](https://github.comgit//DioxusLabs/dioxus/commit/56e7eb83a97ebd6d5bcd23464cfb9d718e5ac26d))
    - buff up html allowed attributes ([`c79d9ae`](https://github.comgit//DioxusLabs/dioxus/commit/c79d9ae674e235c8e9c2c069d24902122b9c7464))
    - it works but the page is backwards ([`cdcd861`](https://github.comgit//DioxusLabs/dioxus/commit/cdcd8611e87ffb5e24de7b9fe6c656af3053276e))
    - use the new structure ([`a05047d`](https://github.comgit//DioxusLabs/dioxus/commit/a05047d01e606425fb0d6595e9d27d3e15f32050))
    - beaf up the use_state hook ([`e4cdb64`](https://github.comgit//DioxusLabs/dioxus/commit/e4cdb645aad800484b19ec35ba1f8bb9ccf71d12))
    - enable arbitrary body in rsx! macro ([`7aec40d`](https://github.comgit//DioxusLabs/dioxus/commit/7aec40d57e78ec13ff3a90ca8149521cbf1d9ff2))
</details>

## v0.1.0 (2021-12-15)

### Documentation

 - <csr-id-285b33dd38c88e2be797eb7f19c34a0906f16b13/> update readme
 - <csr-id-583fdfa5618e11d660985b97e570d4503be2ff49/> big updates to the reference
 - <csr-id-e4c06ce8e893779d2aad0883a1bb27d193bc5985/> update cargo tomls

### New Features

 - <csr-id-11757ddf61e1decb1bd1c2bb30455d0bd01a3e95/> fake bubbling
 - <csr-id-f2234068ba7cd915a00a81e41660d7d6ee1177cc/> events bubble now
 - <csr-id-cfc24f5451cd2d1e9dcd5f1589ee50f705404110/> support innerhtml
 - <csr-id-4a4c7afca7e1beadd4b213214074fdb420eb0923/> shared state mechanisms
 - <csr-id-718fa14b45df38b40b0c0dff7bdc923cba57026b/> a cute crm
 - <csr-id-3bedcb93cacec5bdf134adc38ff02eadbf96c1c6/> svgs working in webview
 - <csr-id-de9f61bcf48c0d6e35e46c337b72a713c9f9f7d2/> more suspended nodes!
 - <csr-id-4091846934b4b3b2bc03d3ca8aaf7712aebd4e36/> add aria
 - <csr-id-c79d9ae674e235c8e9c2c069d24902122b9c7464/> buff up html allowed attributes
 - <csr-id-e4cdb645aad800484b19ec35ba1f8bb9ccf71d12/> beaf up the use_state hook
 - <csr-id-7aec40d57e78ec13ff3a90ca8149521cbf1d9ff2/> enable arbitrary body in rsx! macro

### Bug Fixes

 - <csr-id-601078f9cf78a58d7502a377676ac94f3cf037bf/> desktop and mobile
 - <csr-id-a33f7701fcf5f917fea8719253650b5ad92554fd/> tags
 - <csr-id-868f6739d2b2c5f2ace0c5240cff8008901e818c/> keyword length

