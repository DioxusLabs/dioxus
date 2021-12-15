# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.0.0 (2021-12-15)

### Documentation

 - <csr-id-4de16c4779648e591b3869b5df31271ae603c812/> update local examples and docs to support new syntaxes
 - <csr-id-460783ad549818a85db634ed9c39ffce210b98ec/> lnks to projects
 - <csr-id-583fdfa5618e11d660985b97e570d4503be2ff49/> big updates to the reference
 - <csr-id-70cd46dbb2a689ae2d512e142b8aee9c80798430/> move around examples
 - <csr-id-e4c06ce8e893779d2aad0883a1bb27d193bc5985/> update cargo tomls

### New Features

 - <csr-id-19df1bda109aba03c40ff631263bcb7035004ca0/> bubbling
 - <csr-id-fd93ee89c19b085a04307ef30217170518defa8e/> upgrade syntax
 - <csr-id-f2234068ba7cd915a00a81e41660d7d6ee1177cc/> events bubble now
 - <csr-id-cfc24f5451cd2d1e9dcd5f1589ee50f705404110/> support innerhtml
 - <csr-id-9726a065b0d4fb1ede5b53a2ddd58c855e51539f/> massage lifetimes
 - <csr-id-efd0e9b5648c809057f339083ba9d454f810d483/> support desktop more completely
 - <csr-id-a2b0c50a343005c63c7032bcefb8323b78350bb9/> add update functionality to useref
 - <csr-id-4a4c7afca7e1beadd4b213214074fdb420eb0923/> shared state mechanisms
 - <csr-id-718fa14b45df38b40b0c0dff7bdc923cba57026b/> a cute crm
 - <csr-id-fac42339c272b0e430ebf4f31b6061a0635d3e19/> mutations
 - <csr-id-84b5ddded57238a64e966ec07334e6f9bd86ecf8/> select figured out
 - <csr-id-687cda1b6d9595357d1dc8720ebe921f61098d8f/> re-enable suspense
 - <csr-id-4a72b3140bd244da602deada1eeecded65ff5848/> amazingly awesome error handling
 - <csr-id-d7940aa2ac2017316d62e0f2eac0701dc6ad1f09/> proper handling of events
 - <csr-id-a2c7d17b0595769f60bc1c2bbf7cbe32cec37486/> mvoe away from compound context
 - <csr-id-e5c88fe3a49649ecb308decd14c2557963978619/> make hooks free-functions
 - <csr-id-f782e142118fb7acf1b88a0f3fbb03e4a5e3e91e/> omg what a dumb mistake
 - <csr-id-4837d8e741343d26f31b55e4478a374dc761e538/> suspense!
 - <csr-id-4a0068f09918adbc299150edcf777f342ced0dd3/> bless up, no more segfaults
 - <csr-id-de9f61bcf48c0d6e35e46c337b72a713c9f9f7d2/> more suspended nodes!
 - <csr-id-80e6c256980eb3e8c32e30f3dbb43c8b3b9a9cf4/> move over to push based mechanism
 - <csr-id-e7238762ae518c5688f9339d11832d17f99ad553/> architecture document and edit list
 - <csr-id-abf47596bc2c092bd2f646a77807728416159ad4/> it loads the doggos
 - <csr-id-3a57b942624afb8aa6650aeee05466c3c9ce967e/> task system works
   but I broke the other things :(
 - <csr-id-f457b7113129479cad577237ef21cb735fffe483/> rebuild doesn't return errors
 - <csr-id-9abb0470b7869019d539a2fc21da3872348ae38b/> static node infrastructure and ssr changes
 - <csr-id-7aec40d57e78ec13ff3a90ca8149521cbf1d9ff2/> enable arbitrary body in rsx! macro
 - <csr-id-904b26f7111c3fc66400744ff6192e4b20bf6d74/> add edits back! and more webview support!
   This commit adds a new type - the DomEdit - for serializing the changes made by the diffing machine. The architecture of how DomEdits fit into the cooperative scheduling is still TBD but it will allow us to build change lists without applying them immediately. This is more performant  and allows us to only render parts of the page at a time.
   
   This commit also adds more infrastructure around webview. Dioxus can now run on the web, generate static pages, run in the desktop, and run on mobile, with a large part of thanks to webview.
 - <csr-id-7665f2c6cf05cea64bb9131381d4ac11cbdeb932/> move to slotmap
 - <csr-id-f4fb5bb454536d9f108c7e276ce98a8924ab45e1/> integrate serialization and string borrowing
   This commit adds lifetimes to the diff and realdom methods so consumers may borrow the contents of the DOM for serialization or asynchronous modifications.
 - <csr-id-9d7ee79826a3b3fb952a70abcbb16dcd3363d2fb/> events work again!
 - <csr-id-73047fe95678d50fcfd62a4ace7c6b406c5304e1/> props memoization is more powerful
   This commit solves the memoization , properly memoizing properties that don't have any generic parameters. This is a rough heuristic to prevent non-static lifetimes from creeping into props and breaking our minual lifetime management.
   
   Props that have a generic parameter are opted-out of the `partialeq` requirement and props *without* lifetimes must implement partialeq. We're going to leave manual disabling of memoization for future work.
 - <csr-id-cfa0927cdd40bc3dba22996018605dbad91d0391/> todomvc
 - <csr-id-d4f1ceaffbc0551ea3b179a101885275690cebec/> somewhat working with rc and weak

### Bug Fixes

 - <csr-id-478255f40d4de1d2e3f3cc9b6d758b30ff394b39/> all the bugs!
 - <csr-id-df8aa77198712559f72bef093d064e03b2a5245a/> append isnt backwards
 - <csr-id-a33f7701fcf5f917fea8719253650b5ad92554fd/> tags

### Performance

 - <csr-id-8b3ac0b57ca073c1451e8d5df93882c9360ca52a/> remove global allocation for props

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 177 commits contributed to the release over the course of 334 calendar days.
 - 171 commits where understood as [conventional](https://www.conventionalcommits.org).
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
    - go back to noisy lifetime solution ([`8daf7a6`](https://github.comgit//DioxusLabs/dioxus/commit/8daf7a6ed86df72522b089aa2647eea7bee0f3b6))
    - clean up the core crate ([`e6c6bbd`](https://github.comgit//DioxusLabs/dioxus/commit/e6c6bbdc1ec6a8c251b78c05ca104f006b6fad26))
    - rename fc to component ([`1e4a599`](https://github.comgit//DioxusLabs/dioxus/commit/1e4a599d14af85a2d1c29a442dd489f8dc8df321))
    - bubbling ([`19df1bd`](https://github.comgit//DioxusLabs/dioxus/commit/19df1bda109aba03c40ff631263bcb7035004ca0))
    - some docs and suspense ([`93d4b8c`](https://github.comgit//DioxusLabs/dioxus/commit/93d4b8ca7c1b133e5dba2a8dc9a310dbe1357001))
    - move examples around ([`1e6e5e6`](https://github.comgit//DioxusLabs/dioxus/commit/1e6e5e611b61571f272289adefc9cdd7d59c4399))
    - docs and router ([`a5f05d7`](https://github.comgit//DioxusLabs/dioxus/commit/a5f05d73acc0e47b05cff64a373482519414bc7c))
    - upgrade syntax ([`fd93ee8`](https://github.comgit//DioxusLabs/dioxus/commit/fd93ee89c19b085a04307ef30217170518defa8e))
    - events bubble now ([`f223406`](https://github.comgit//DioxusLabs/dioxus/commit/f2234068ba7cd915a00a81e41660d7d6ee1177cc))
    - support innerhtml ([`cfc24f5`](https://github.comgit//DioxusLabs/dioxus/commit/cfc24f5451cd2d1e9dcd5f1589ee50f705404110))
    - massage lifetimes ([`9726a06`](https://github.comgit//DioxusLabs/dioxus/commit/9726a065b0d4fb1ede5b53a2ddd58c855e51539f))
    - move everything over to a stack dst ([`0e9d5fc`](https://github.comgit//DioxusLabs/dioxus/commit/0e9d5fc5306ab508d5af6999a4064f9b8b48460f))
    - event system ([`1f22a06`](https://github.comgit//DioxusLabs/dioxus/commit/1f22a06a36f72073188b8c9009dd4950b3f4ff9a))
    - support desktop more completely ([`efd0e9b`](https://github.comgit//DioxusLabs/dioxus/commit/efd0e9b5648c809057f339083ba9d454f810d483))
    - add update functionality to useref ([`a2b0c50`](https://github.comgit//DioxusLabs/dioxus/commit/a2b0c50a343005c63c7032bcefb8323b78350bb9))
    - lnks to projects ([`460783a`](https://github.comgit//DioxusLabs/dioxus/commit/460783ad549818a85db634ed9c39ffce210b98ec))
    - overhaul event system ([`7a03c1d`](https://github.comgit//DioxusLabs/dioxus/commit/7a03c1d2b48590276b182465679387655fe08f3a))
    - threadsafe ([`82953f2`](https://github.comgit//DioxusLabs/dioxus/commit/82953f2ac37913f83a822333acd0c47e20777d31))
    - all the bugs! ([`478255f`](https://github.comgit//DioxusLabs/dioxus/commit/478255f40d4de1d2e3f3cc9b6d758b30ff394b39))
    - ssr ([`71f0df6`](https://github.comgit//DioxusLabs/dioxus/commit/71f0df63745fe5c17468693144c552ea3a0a7101))
    - shared state mechanisms ([`4a4c7af`](https://github.comgit//DioxusLabs/dioxus/commit/4a4c7afca7e1beadd4b213214074fdb420eb0923))
    - fix web list issue ([`da4423c`](https://github.comgit//DioxusLabs/dioxus/commit/da4423c141f1f376df5f3f2580e5284831744a7e))
    - move macro crate out of core ([`7bdad1e`](https://github.comgit//DioxusLabs/dioxus/commit/7bdad1e2e6f67e74c9f67dde2150140cf8a090e8))
    - clean up to web ([`b43a964`](https://github.comgit//DioxusLabs/dioxus/commit/b43a964f982eb871195278a093cc214c3a8ad66d))
    - clean up the web module ([`823adc0`](https://github.comgit//DioxusLabs/dioxus/commit/823adc0834b581327aee745c72ce8993f0bba5aa))
    - slightly simplify crm ([`f07e345`](https://github.comgit//DioxusLabs/dioxus/commit/f07e345eb2a2e4197270396dffebc85c65272fe0))
    - fix some event stuff for web and core ([`725b4a1`](https://github.comgit//DioxusLabs/dioxus/commit/725b4a1d7f5d629b1b0a163b65bfd93b2f8a151b))
    - a cute crm ([`718fa14`](https://github.comgit//DioxusLabs/dioxus/commit/718fa14b45df38b40b0c0dff7bdc923cba57026b))
    - on collaborative scheduling ([`1a32383`](https://github.comgit//DioxusLabs/dioxus/commit/1a323835c8c4f667e5744bbf5447776f1dc51fca))
    - examples ([`1a2f91e`](https://github.comgit//DioxusLabs/dioxus/commit/1a2f91ed91c13dae553ecde585462ab261b1b95d))
    - mutations ([`fac4233`](https://github.comgit//DioxusLabs/dioxus/commit/fac42339c272b0e430ebf4f31b6061a0635d3e19))
    - performance looks good, needs more testing ([`4b6ca05`](https://github.comgit//DioxusLabs/dioxus/commit/4b6ca05f2c3ad647842c858967da9c87f1915825))
    - add test_dom ([`a652090`](https://github.comgit//DioxusLabs/dioxus/commit/a652090dc5708db334fa7430fededb1bac207880))
    - bottom up dropping ([`f2334c1`](https://github.comgit//DioxusLabs/dioxus/commit/f2334c17be2612d926361686d7d40a57e3ffe9b9))
    - cleanup ([`1745a44`](https://github.comgit//DioxusLabs/dioxus/commit/1745a44d949b994b64ea1fb715cbe36963ae7027))
    - select figured out ([`84b5ddd`](https://github.comgit//DioxusLabs/dioxus/commit/84b5ddded57238a64e966ec07334e6f9bd86ecf8))
    - re-enable suspense ([`687cda1`](https://github.comgit//DioxusLabs/dioxus/commit/687cda1b6d9595357d1dc8720ebe921f61098d8f))
    - fill out the snippets ([`6051b0e`](https://github.comgit//DioxusLabs/dioxus/commit/6051b0ec86927704451f4ce6cdf8f988e59702ae))
    - amazingly awesome error handling ([`4a72b31`](https://github.comgit//DioxusLabs/dioxus/commit/4a72b3140bd244da602deada1eeecded65ff5848))
    - some ideas ([`05c909f`](https://github.comgit//DioxusLabs/dioxus/commit/05c909f320765aec1bf4c1c55ca59ffd5525a2c7))
    - proper handling of events ([`d7940aa`](https://github.comgit//DioxusLabs/dioxus/commit/d7940aa2ac2017316d62e0f2eac0701dc6ad1f09))
    - websys dom working properly ([`cfa0247`](https://github.comgit//DioxusLabs/dioxus/commit/cfa0247cbb1233e1df275374a73f431650a9250f))
    - big updates to the reference ([`583fdfa`](https://github.comgit//DioxusLabs/dioxus/commit/583fdfa5618e11d660985b97e570d4503be2ff49))
    - broken diff ([`f15e1fa`](https://github.comgit//DioxusLabs/dioxus/commit/f15e1fa892070f9cf1fc627eb9c36fbaa0dc309b))
    - cleanup workspace ([`8f0bb5d`](https://github.comgit//DioxusLabs/dioxus/commit/8f0bb5dc5bfa3e775af567c4b569622cdd932af1))
    - changes to scheduler ([`098d382`](https://github.comgit//DioxusLabs/dioxus/commit/098d3821ed89ad38d99077a6556b48a7e91fc3fc))
    - web stuff ([`acad9ca`](https://github.comgit//DioxusLabs/dioxus/commit/acad9ca622748f96599dd02ad22aaeaae3621b76))
    - making progress on diffing and hydration ([`49856cc`](https://github.comgit//DioxusLabs/dioxus/commit/49856ccd6865f88d63765f26d27f7e945b554da0))
    - mvoe away from compound context ([`a2c7d17`](https://github.comgit//DioxusLabs/dioxus/commit/a2c7d17b0595769f60bc1c2bbf7cbe32cec37486))
    - make hooks free-functions ([`e5c88fe`](https://github.comgit//DioxusLabs/dioxus/commit/e5c88fe3a49649ecb308decd14c2557963978619))
    - more work on suspense and documentation ([`37ed4be`](https://github.comgit//DioxusLabs/dioxus/commit/37ed4bed8cf28eb65465d41e15e8d758cb3d9679))
    - omg what a dumb mistake ([`f782e14`](https://github.comgit//DioxusLabs/dioxus/commit/f782e142118fb7acf1b88a0f3fbb03e4a5e3e91e))
    - suspense! ([`4837d8e`](https://github.comgit//DioxusLabs/dioxus/commit/4837d8e741343d26f31b55e4478a374dc761e538))
    - refactor ([`8b0eb87`](https://github.comgit//DioxusLabs/dioxus/commit/8b0eb87c72ea9d444dee99a8b05643f19fea2634))
    - bless up, no more segfaults ([`4a0068f`](https://github.comgit//DioxusLabs/dioxus/commit/4a0068f09918adbc299150edcf777f342ced0dd3))
    - more suspended nodes! ([`de9f61b`](https://github.comgit//DioxusLabs/dioxus/commit/de9f61bcf48c0d6e35e46c337b72a713c9f9f7d2))
    - more docs ([`34c3107`](https://github.comgit//DioxusLabs/dioxus/commit/34c3107418df89620cb9dfbd9300f89a1fe6e68f))
    - basic support for scheduled rendering ([`c52af22`](https://github.comgit//DioxusLabs/dioxus/commit/c52af221f755601a9e826ffc2c355def138999d0))
    - move over to push based mechanism ([`80e6c25`](https://github.comgit//DioxusLabs/dioxus/commit/80e6c256980eb3e8c32e30f3dbb43c8b3b9a9cf4))
    - solve some issues regarding listeners ([`dfaf5ad`](https://github.comgit//DioxusLabs/dioxus/commit/dfaf5adee164f44a679ab21d730caaab3610e01f))
    - architecture document and edit list ([`e723876`](https://github.comgit//DioxusLabs/dioxus/commit/e7238762ae518c5688f9339d11832d17f99ad553))
    - move to slab ([`6084fbc`](https://github.comgit//DioxusLabs/dioxus/commit/6084fbcd11d1669db135098d920488e2c01a7014))
    - change in cx to cx ([`9971ff2`](https://github.comgit//DioxusLabs/dioxus/commit/9971ff215db6f771b7ec1cae2517c85d47d38622))
    - move things into a "shared" object ([`f644d7c`](https://github.comgit//DioxusLabs/dioxus/commit/f644d7c44159eef091552dcc90acbb151ea76b21))
    - it loads the doggos ([`abf4759`](https://github.comgit//DioxusLabs/dioxus/commit/abf47596bc2c092bd2f646a77807728416159ad4))
    - task system works ([`3a57b94`](https://github.comgit//DioxusLabs/dioxus/commit/3a57b942624afb8aa6650aeee05466c3c9ce967e))
    - more overhaul on virtualevents ([`41cc429`](https://github.comgit//DioxusLabs/dioxus/commit/41cc42919d42453f8f2560aa852211364af4ad3d))
    - more work on web ([`3bf19d8`](https://github.comgit//DioxusLabs/dioxus/commit/3bf19d8106e0cca7749b7346e9b0adfce654d782))
    - ricraf polyfill ([`59219b9`](https://github.comgit//DioxusLabs/dioxus/commit/59219b9ef22caa5896046ccaabc8a225cc4b5a53))
    - ....sigh..... so the diffing algorithm is robust ([`68ed1c0`](https://github.comgit//DioxusLabs/dioxus/commit/68ed1c04e7e773f9e6c0a5148f0ea89b97b6784e))
    - remove global allocation for props ([`8b3ac0b`](https://github.comgit//DioxusLabs/dioxus/commit/8b3ac0b57ca073c1451e8d5df93882c9360ca52a))
    - ric_raf wired up ([`8b0d04c`](https://github.comgit//DioxusLabs/dioxus/commit/8b0d04ce585596f561f15e92fc087e2c7063eb07))
    - rebuild doesn't return errors ([`f457b71`](https://github.comgit//DioxusLabs/dioxus/commit/f457b7113129479cad577237ef21cb735fffe483))
    - more examples ([`56e7eb8`](https://github.comgit//DioxusLabs/dioxus/commit/56e7eb83a97ebd6d5bcd23464cfb9d718e5ac26d))
    - tests and documentation ([`da81591`](https://github.comgit//DioxusLabs/dioxus/commit/da8159190b84d13caa9dfe7dea385a809cf84199))
    - append isnt backwards ([`df8aa77`](https://github.comgit//DioxusLabs/dioxus/commit/df8aa77198712559f72bef093d064e03b2a5245a))
    - it works but the page is backwards ([`cdcd861`](https://github.comgit//DioxusLabs/dioxus/commit/cdcd8611e87ffb5e24de7b9fe6c656af3053276e))
    - static node infrastructure and ssr changes ([`9abb047`](https://github.comgit//DioxusLabs/dioxus/commit/9abb0470b7869019d539a2fc21da3872348ae38b))
    - more refactor for async ([`975fa56`](https://github.comgit//DioxusLabs/dioxus/commit/975fa566f9809f8fa2bb0bdb07fbfc7f855dcaeb))
    - enable arbitrary body in rsx! macro ([`7aec40d`](https://github.comgit//DioxusLabs/dioxus/commit/7aec40d57e78ec13ff3a90ca8149521cbf1d9ff2))
    - working on async diff ([`f41cff5`](https://github.comgit//DioxusLabs/dioxus/commit/f41cff571fd11bb00820c0bf9d147e81ed5a6a53))
    - move some examples around ([`98a0933`](https://github.comgit//DioxusLabs/dioxus/commit/98a09339fd3190799ea4dd316908f0a53fdf2413))
    - fix issues with lifetimes ([`a38a81e`](https://github.comgit//DioxusLabs/dioxus/commit/a38a81e1290375cae685f7c49d3745e4298fab26))
    - groundwork for noderefs ([`c1afeba`](https://github.comgit//DioxusLabs/dioxus/commit/c1afeba1efb1a063705466a14648beee08cacb86))
    - more examples ([`11f89e5`](https://github.comgit//DioxusLabs/dioxus/commit/11f89e5d338d14a7aeece0a6275c24ae65913ce7))
    - add edits back! and more webview support! ([`904b26f`](https://github.comgit//DioxusLabs/dioxus/commit/904b26f7111c3fc66400744ff6192e4b20bf6d74))
    - enable more diffing ([`e8f29a8`](https://github.comgit//DioxusLabs/dioxus/commit/e8f29a8f8ac56020bee0048021efa52547307a77))
    - example ([`eb39b00`](https://github.comgit//DioxusLabs/dioxus/commit/eb39b000d7f0ef16d0b83ec44d3926f4690fd08f))
    - wip ([`952a91d`](https://github.comgit//DioxusLabs/dioxus/commit/952a91d5408aaf789b496f11d01c3b3f7fcf9059))
    - integrate signals ([`93900aa`](https://github.comgit//DioxusLabs/dioxus/commit/93900aac4443db2e63631ed928294af74201a9ff))
    - move to slotmap ([`7665f2c`](https://github.comgit//DioxusLabs/dioxus/commit/7665f2c6cf05cea64bb9131381d4ac11cbdeb932))
    - integrate serialization and string borrowing ([`f4fb5bb`](https://github.comgit//DioxusLabs/dioxus/commit/f4fb5bb454536d9f108c7e276ce98a8924ab45e1))
    - more work on diffing machine ([`9813f23`](https://github.comgit//DioxusLabs/dioxus/commit/9813f23cdf7b32bc058771c91db3f182d6224905))
    - rename ctx to cx ([`81382e7`](https://github.comgit//DioxusLabs/dioxus/commit/81382e7044fb3dba61d4abb1e6086b7b29143116))
    - move around examples ([`70cd46d`](https://github.comgit//DioxusLabs/dioxus/commit/70cd46dbb2a689ae2d512e142b8aee9c80798430))
    - start moving events to rc<event> ([`b9ff95f`](https://github.comgit//DioxusLabs/dioxus/commit/b9ff95fa12c46365fe73b64a4926a506d5da2342))
    - rename recoil to atoms ([`36ea39a`](https://github.comgit//DioxusLabs/dioxus/commit/36ea39ae30aa3f1fb2d718c0fdf08850c6bfd3ac))
    - more examples and docs ([`7fbaf69`](https://github.comgit//DioxusLabs/dioxus/commit/7fbaf69cabbdde712bb3fd9e4b2a5dc18b9390e9))
    - more work on updating syntad ([`47e8960`](https://github.comgit//DioxusLabs/dioxus/commit/47e896038ef3655566f3eda83d1d2adfefbc8862))
    - some cleanup and documentation ([`517d7f1`](https://github.comgit//DioxusLabs/dioxus/commit/517d7f14957c4dae9fc894bfbdcd00a955d09f20))
    - docs ([`f5683a2`](https://github.comgit//DioxusLabs/dioxus/commit/f5683a23464992ecace463a61414795b5a2c58c8))
    - pre vnodes instead of vnode ([`fe6938c`](https://github.comgit//DioxusLabs/dioxus/commit/fe6938ceb3dba0796ae8bab52ae41248dc0d3650))
    - events work again! ([`9d7ee79`](https://github.comgit//DioxusLabs/dioxus/commit/9d7ee79826a3b3fb952a70abcbb16dcd3363d2fb))
    - props memoization is more powerful ([`73047fe`](https://github.comgit//DioxusLabs/dioxus/commit/73047fe95678d50fcfd62a4ace7c6b406c5304e1))
    - massive changes to definition of components ([`508c560`](https://github.comgit//DioxusLabs/dioxus/commit/508c560320d78730fa058156421523ffa5695d9d))
    - moving to IDs ([`79127ea`](https://github.comgit//DioxusLabs/dioxus/commit/79127ea6cdabf62bf91e777aaacb563e3aa5e619))
    - move to static props ([`c1fd848`](https://github.comgit//DioxusLabs/dioxus/commit/c1fd848f89b0146581d8e485fa0d4a847387b963))
    - doesnt share on thread ([`fe67ff9`](https://github.comgit//DioxusLabs/dioxus/commit/fe67ff9fa4c9d5009670c922e192dccedb7cd09a))
    - parity document ([`ba97541`](https://github.comgit//DioxusLabs/dioxus/commit/ba975410f9fe2a5b3a0e407f6a2478bf86577f29))
    - recoil ([`ee67654`](https://github.comgit//DioxusLabs/dioxus/commit/ee67654f58aecca91c640fc49451d0b99bc05981))
    - buff the readme and docs ([`3cfa1fe`](https://github.comgit//DioxusLabs/dioxus/commit/3cfa1fe125886787f35905ed9b05340a739bc654))
    - Todomvc in progress ([`b843dbd`](https://github.comgit//DioxusLabs/dioxus/commit/b843dbd3679abf86a34347d87fd4ce5fe9e2aca5))
    - introduce children for walking down the tree ([`0d44f00`](https://github.comgit//DioxusLabs/dioxus/commit/0d44f009b0176cb9cf8203374cf534f5af7de63c))
    - Clean up repo a bit ([`a99147c`](https://github.comgit//DioxusLabs/dioxus/commit/a99147c85b53b4ee336a94deee463d793cebf572))
    - some code health ([`c28697e`](https://github.comgit//DioxusLabs/dioxus/commit/c28697e1fe3136d1835f2b663715f34aab9f4b17))
    - major overhaul to diffing ([`9810fee`](https://github.comgit//DioxusLabs/dioxus/commit/9810feebf57f93114e3d7faf6de053ac192593a9))
    - Wip ([`c809095`](https://github.comgit//DioxusLabs/dioxus/commit/c809095124d06175e7f49853c34e48d7335573c5))
    - refactor a bit ([`2eeb8f2`](https://github.comgit//DioxusLabs/dioxus/commit/2eeb8f2386409b53836fab687097d89af7b883c3))
    - todos ([`8c541f6`](https://github.comgit//DioxusLabs/dioxus/commit/8c541f66d5f7ef2286f2cdf9b0496a9c404471f9))
    - todomvc ([`cfa0927`](https://github.comgit//DioxusLabs/dioxus/commit/cfa0927cdd40bc3dba22996018605dbad91d0391))
    - todomvc ([`ce33031`](https://github.comgit//DioxusLabs/dioxus/commit/ce33031519fbbbd207f1dffb75acf62bf59e3c9e))
    - more ergonomics, more examples ([`0bcff1f`](https://github.comgit//DioxusLabs/dioxus/commit/0bcff1f88e4b1a633b7a9b7c6c2e39b8bd3666c4))
    - building large apps, revamp macro ([`9f7f43b`](https://github.comgit//DioxusLabs/dioxus/commit/9f7f43b6614aaef2d7dded7058e81934f28f5dec))
    - more liveview and webview custom client ([`9b560df`](https://github.comgit//DioxusLabs/dioxus/commit/9b560dfedb988b258f5c564986759cb83730a96c))
    - livehost bones ([`7856f2b`](https://github.comgit//DioxusLabs/dioxus/commit/7856f2b1537b52478a6fa1ca55d0a8c5793a91e5))
    - some stuff related to event listeners. POC for lifecyel ([`5b7887d`](https://github.comgit//DioxusLabs/dioxus/commit/5b7887d76c714d11e0cea5207197ffaac856a0a7))
    - diffing approach slightly broken ([`4e48e05`](https://github.comgit//DioxusLabs/dioxus/commit/4e48e0514e1caed9f60bcac4048114008ac76439))
    - remove old macro ([`9d0727e`](https://github.comgit//DioxusLabs/dioxus/commit/9d0727edabbf759dac8a3cffabd7103d08b728c1))
    - add deeply neste example ([`e66827e`](https://github.comgit//DioxusLabs/dioxus/commit/e66827ec92e23bf0602f92d5223894a7831dfd0f))
    - update examples ([`39bd185`](https://github.comgit//DioxusLabs/dioxus/commit/39bd1856f4d7c2364d6ca2c7812ab5d6e71252b9))
    - ensure mutabality is okay when not double-using the components ([`305ff91`](https://github.comgit//DioxusLabs/dioxus/commit/305ff919effa919ff7ea5db2a71cf21eca106588))
    - somewhat working with rc and weak ([`d4f1cea`](https://github.comgit//DioxusLabs/dioxus/commit/d4f1ceaffbc0551ea3b179a101885275690cebec))
    - foregin eq from comparapable comp type. ([`ec801ea`](https://github.comgit//DioxusLabs/dioxus/commit/ec801eab167f9be5325ab877e74e2b65d501e129))
    - staticify? ([`5ad8188`](https://github.comgit//DioxusLabs/dioxus/commit/5ad81885e499bf02ac79e0098f7956d02ee5f2e5))
    - Feat:  it's awersome ([`8dc2619`](https://github.comgit//DioxusLabs/dioxus/commit/8dc26195e274874a7de6c806372f9f77a5c82c5d))
    - yeet, synthetic somewhat wired up ([`d959806`](https://github.comgit//DioxusLabs/dioxus/commit/d9598066c2679d9d0b9ca0ce1d3f26110a238cd2))
    - remove FC ([`92d9521`](https://github.comgit//DioxusLabs/dioxus/commit/92d9521a73aefb620b354ae5954617109dd06e7e))
    - more cleanup ([`5a9155b`](https://github.comgit//DioxusLabs/dioxus/commit/5a9155b059acc1fb3c8b8accbeca3701ce4f0ab6))
    - add context to builder ([`cf16090`](https://github.comgit//DioxusLabs/dioxus/commit/cf16090838d127354e333dcbc0b06474835b87d6))
    - listeners now have scope information ([`fcd68e6`](https://github.comgit//DioxusLabs/dioxus/commit/fcd68e61d2400628469ba193b009e7bf1fd3acdf))
    - broken, but solved ([`cb74d70`](https://github.comgit//DioxusLabs/dioxus/commit/cb74d70f831b5510f1ee191d91eaff621ffa6256))
    - accept closures directly in handler ([`f225030`](https://github.comgit//DioxusLabs/dioxus/commit/f225030506967415a21f4af0372477cb5224ee7c))
    - bug that phantom triggered events ([`07f671c`](https://github.comgit//DioxusLabs/dioxus/commit/07f671c8e19a2a2e90afc3db94c4edebcfffe982))
    - wowza got it all working ([`4b8e9f4`](https://github.comgit//DioxusLabs/dioxus/commit/4b8e9f4a125b9d55439d919786f33d9d5df234e8))
    - update readme and examples ([`ffaf687`](https://github.comgit//DioxusLabs/dioxus/commit/ffaf6878963981860089c2362947bf77a84c9058))
    - view -> render ([`c8bb392`](https://github.comgit//DioxusLabs/dioxus/commit/c8bb392cadb22ddb41e53a776ab0677945579a9c))
    - bump core version ([`6fabd8c`](https://github.comgit//DioxusLabs/dioxus/commit/6fabd8ccc87c0b30a95693e85b98bdb4e2b1a936))
    - update and prep for dioxusweb ([`ab655ea`](https://github.comgit//DioxusLabs/dioxus/commit/ab655eac97fcd9ea3542edbdea4c6cf9a06956fd))
    - a few bugs, but the event system works! ([`3b30fa6`](https://github.comgit//DioxusLabs/dioxus/commit/3b30fa61b8f1927f22ffec373a3405ddfdefde39))
    - moving to CbIdx as serializable event system ([`e840f47`](https://github.comgit//DioxusLabs/dioxus/commit/e840f472faf25d8e1d65abeb610daed6a522771d))
    - custom format_args for inlining variables into html templates ([`e4b1f6e`](https://github.comgit//DioxusLabs/dioxus/commit/e4b1f6ea0d0db707cf757dabf8635e9fc91a3e0f))
    - begin WIP on html macro ([`a8b1225`](https://github.comgit//DioxusLabs/dioxus/commit/a8b1225c4853a61c7862e1a33eba64b8ed4e06d5))
    - move webview logic into library ([`32b45e5`](https://github.comgit//DioxusLabs/dioxus/commit/32b45e5ba168390e338a343eef6baba5cca9468b))
    - comments ([`18a7a1f`](https://github.comgit//DioxusLabs/dioxus/commit/18a7a1f9c40eb3b1ac263cbd21b1daa6da9c7093))
    - buff up examples and docs ([`8d3e2ad`](https://github.comgit//DioxusLabs/dioxus/commit/8d3e2ade7aef491921bb51f0d35b7253bbc800f8))
    - wire up rebuild ([`06ae4fc`](https://github.comgit//DioxusLabs/dioxus/commit/06ae4fc17833ff9695b0645f940b5ea32eeb4ef0))
    - update websys with lifecycle ([`4d01455`](https://github.comgit//DioxusLabs/dioxus/commit/4d01455729dac55cf049c50f47a511190d13550e))
    - fix internal lifecycle ([`5204862`](https://github.comgit//DioxusLabs/dioxus/commit/5204862bc220e55afe8d2f8f4422cb530a6ea1cf))
    - add css example ([`edf09c1`](https://github.comgit//DioxusLabs/dioxus/commit/edf09c1892f2521f41bf84372629e84bed86e921))
    - WIP ctx ([`7a6aabe`](https://github.comgit//DioxusLabs/dioxus/commit/7a6aabe4f39aa38bec7b548d030c11b2d9f481cb))
    - desktop app wired up ([`b3e6886`](https://github.comgit//DioxusLabs/dioxus/commit/b3e68863514a33291556ce278b7255fdf53e8b50))
    - re-enable stack machine approach ([`e3ede7f`](https://github.comgit//DioxusLabs/dioxus/commit/e3ede7fcbf7acb5d8762bcddb5f1499104bb0ce7))
    - WIP on deserialize ([`f22ff83`](https://github.comgit//DioxusLabs/dioxus/commit/f22ff8319078d283e675ff8dc735da5bf8efe0df))
    - web example + cli writes to browser screen! ([`8439994`](https://github.comgit//DioxusLabs/dioxus/commit/84399948596c573883bd29786edee48f5d4ef438))
    - wire up a very basic dom updater ([`c4e8d8b`](https://github.comgit//DioxusLabs/dioxus/commit/c4e8d8bb31d54d70c8da2f4a5e3e3926ff7df92b))
    - major overhaul to diffing, using a "diffing machine" now ([`4dfdf91`](https://github.comgit//DioxusLabs/dioxus/commit/4dfdf9123608c69b86a56acbd6d8810b0cf1918c))
    - remove generic paramter on VDOM ([`4c291a0`](https://github.comgit//DioxusLabs/dioxus/commit/4c291a0efdbaadf5c2212248367c0a78046e2f83))
    - wire up some of the changelist for diff ([`d063a19`](https://github.comgit//DioxusLabs/dioxus/commit/d063a199391383288e807ac03d333091bac6ba60))
    - event loop ([`ea2aa4b`](https://github.comgit//DioxusLabs/dioxus/commit/ea2aa4b0c97b3292178284e05756d1415902a9e2))
    - Dioxus-webview ([`9c01736`](https://github.comgit//DioxusLabs/dioxus/commit/9c0173689539210d14847613f9a1694e6cb34506))
    - more docs, dissolve vnode crate into dioxus-core ([`9c616ea`](https://github.comgit//DioxusLabs/dioxus/commit/9c616ea5c092a756a437ccf175c8b04ada50b1b6))
    - WIP ([`ce34d0d`](https://github.comgit//DioxusLabs/dioxus/commit/ce34d0dfcda82d847a6032f8f42749b39e0fba18))
</details>

