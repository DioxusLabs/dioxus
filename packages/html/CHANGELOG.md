# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 36 commits contributed to the release over the course of 160 calendar days.
 - 33 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - update cargo tomls ([`e4c06ce`](https://github.comgit//DioxusLabs/dioxus/commit/e4c06ce8e893779d2aad0883a1bb27d193bc5985))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`270dfc9`](https://github.comgit//DioxusLabs/dioxus/commit/270dfc9590b2354d083ea8da5cc0e1a1497d30e0))
    - bump versions ([`0846d93`](https://github.comgit//DioxusLabs/dioxus/commit/0846d93d41c27464ca271757c6f24a2cef8fb997))
    - rename fc to component ([`1e4a599`](https://github.comgit//DioxusLabs/dioxus/commit/1e4a599d14af85a2d1c29a442dd489f8dc8df321))
    - polish ([`8bf57dc`](https://github.comgit//DioxusLabs/dioxus/commit/8bf57dc21dfbcbae5b95650203b68d3f41227652))
    - docs and router ([`a5f05d7`](https://github.comgit//DioxusLabs/dioxus/commit/a5f05d73acc0e47b05cff64a373482519414bc7c))
    - fake bubbling ([`11757dd`](https://github.comgit//DioxusLabs/dioxus/commit/11757ddf61e1decb1bd1c2bb30455d0bd01a3e95))
    - events bubble now ([`f223406`](https://github.comgit//DioxusLabs/dioxus/commit/f2234068ba7cd915a00a81e41660d7d6ee1177cc))
    - Merge branch 'master' of https://github.com/jkelleyrtp/dioxus ([`60d6eb2`](https://github.comgit//DioxusLabs/dioxus/commit/60d6eb204a10633e5e52f91e855bd12c5cda40f2))
    - desktop and mobile ([`601078f`](https://github.comgit//DioxusLabs/dioxus/commit/601078f9cf78a58d7502a377676ac94f3cf037bf))
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

