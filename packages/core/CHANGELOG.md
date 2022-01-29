# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Documentation

 - <csr-id-036a0ff49a7dade0e04c9c07071a1ff49133ee24/> add comments for the Handler

### New Features

 - <csr-id-bad4b773b7bbab6a38d4f48d88d8c5a8787927ac/> add "spawn" method
 - <csr-id-d2bd1751436ef4bec554bb361ba87aea1357036a/> allow providing context to the root component

### Bug Fixes

 - <csr-id-d9a07ddddb45ba00e35645ac0e0ad1a4d2dd996d/> provide_root_context on root scopes
 - <csr-id-c8d528b3b18e320ca0cee1542ae5fc659c1cca81/> proprogation of root context
 - <csr-id-e47ead5347ce778935f8f2127fcb14f520eda0ce/> allow eventhandler to derive default

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 18 commits contributed to the release over the course of 13 calendar days.
 - 8 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #169 from DioxusLabs/jk/router-userouter ([`3509602`](https://github.comgit//DioxusLabs/dioxus/commit/3509602c0bcd327a33bc8c95896775e24751da1a))
    - Merge branch 'master' of github.com:DioxusLabs/dioxus ([`8899701`](https://github.comgit//DioxusLabs/dioxus/commit/88997019c5c74658bfd76344d6bb6f3565943b41))
    - add miri stress test ([`e9792e9`](https://github.comgit//DioxusLabs/dioxus/commit/e9792e9b95521fcf0b0b4e240d7cf0397cd79b8f))
    - add "spawn" method ([`bad4b77`](https://github.comgit//DioxusLabs/dioxus/commit/bad4b773b7bbab6a38d4f48d88d8c5a8787927ac))
    - drop hooks before resetting bump arena ([`2e4f765`](https://github.comgit//DioxusLabs/dioxus/commit/2e4f7659327b69b74e20af59b6d5e65c699b6adb))
    - provide_root_context on root scopes ([`d9a07dd`](https://github.comgit//DioxusLabs/dioxus/commit/d9a07ddddb45ba00e35645ac0e0ad1a4d2dd996d))
    - Fix various typos and grammar nits ([`9e4ec43`](https://github.comgit//DioxusLabs/dioxus/commit/9e4ec43b1e78d355c56a38e4c092170b2b01b20d))
    - Merge pull request #121 from DioxusLabs/jk/unify ([`b287a4c`](https://github.comgit//DioxusLabs/dioxus/commit/b287a4cab3eec9e6961a3f010846291f4f105747))
    - Merge pull request #108 from DioxusLabs/jk/fstring-component-fields ([`f4132d1`](https://github.comgit//DioxusLabs/dioxus/commit/f4132d1874f7495049fac23ba0a022ac137ad74f))
    - proprogation of root context ([`c8d528b`](https://github.comgit//DioxusLabs/dioxus/commit/c8d528b3b18e320ca0cee1542ae5fc659c1cca81))
    - allow providing context to the root component ([`d2bd175`](https://github.comgit//DioxusLabs/dioxus/commit/d2bd1751436ef4bec554bb361ba87aea1357036a))
    - Enable clippy ([`b6903bf`](https://github.comgit//DioxusLabs/dioxus/commit/b6903bf558bc7a3d0fe6794a137c44fca0957d11))
    - Merge pull request #107 from autarch/autarch/half-assed-router ([`8d3ac3f`](https://github.comgit//DioxusLabs/dioxus/commit/8d3ac3ff148aef9d10a393eda453a11c1e882f58))
    - Merge pull request #138 from mrxiaozhuox/master ([`8c7473d`](https://github.comgit//DioxusLabs/dioxus/commit/8c7473d1943dd133f388ec36116c9d8295861b97))
    - Merge pull request #133 from mrxiaozhuox/master ([`887f69d`](https://github.comgit//DioxusLabs/dioxus/commit/887f69d5b47bdcde4fe0eab094c0cd0de23e4f3f))
    - Don't expect all components to have a scope in ScopeArena.ensure_drop_safety ([`9b282d8`](https://github.comgit//DioxusLabs/dioxus/commit/9b282d877ba0fe2463687cde7ea899b7f8510425))
    - add comments for the Handler ([`036a0ff`](https://github.comgit//DioxusLabs/dioxus/commit/036a0ff49a7dade0e04c9c07071a1ff49133ee24))
    - allow eventhandler to derive default ([`e47ead5`](https://github.comgit//DioxusLabs/dioxus/commit/e47ead5347ce778935f8f2127fcb14f520eda0ce))
</details>

## v0.1.7 (2022-01-08)

### Documentation

 - <csr-id-d11f322f554e7dbf43b988c9cfda56498cc49872/> add title to doc comment
 - <csr-id-be9f1a52ad2b04f101397ae34482ea7394df653b/> better document the `EventHandler` type
 - <csr-id-9874f342e2bc1116410142c691545d75b0e0d6fe/> more docs
 - <csr-id-54d050cf897131ddc4f53651f1aeedaf03f23f57/> strong improvmenets, first major section done
 - <csr-id-e4c06ce8e893779d2aad0883a1bb27d193bc5985/> update cargo tomls
 - <csr-id-9b5f82af7dce18b75a781b86b188bc713860bb99/> start on components
 - <csr-id-4de16c4779648e591b3869b5df31271ae603c812/> update local examples and docs to support new syntaxes
 - <csr-id-e495b09bf1c99950b5ea8647b371f296dc64afbe/> fix table
 - <csr-id-460783ad549818a85db634ed9c39ffce210b98ec/> lnks to projects
 - <csr-id-583fdfa5618e11d660985b97e570d4503be2ff49/> big updates to the reference
 - <csr-id-d9e6d0925b30690212d1d690dfba288f1a694a27/> examples
 - <csr-id-0826fdfee13d26b08cb8a2fa45cbd59cc4f20c25/> more docs
 - <csr-id-70cd46dbb2a689ae2d512e142b8aee9c80798430/> move around examples
 - <csr-id-69f5cc3802af136729bc73e5c3d209270d41b184/> move into a fromjs tutorial
 - <csr-id-7102fe5f984fe8692cf977ea3a43e2973eed4f45/> add some more sources in the core implementation

### New Features

 - <csr-id-427b126bc17336d5d14d56eb7fddb8e07752495f/> add prevent default attribute and upgrade router
 - <csr-id-bbb6ee10de824f2e3259576ac01768640c884279/> make hydration more robust
 - <csr-id-06276edd0d4f1f6f6b0a3bf7a467931413ab33c3/> eanble bubbling
 - <csr-id-d84fc0538670b2a3bda9ae41878896793b74e8ee/> plug in bubbling
 - <csr-id-a4f280d16399205c638033bf9beb858e478e98ff/> more API updates
 - <csr-id-b997b8ebbb82b5b9e9119bd2eb25335e2ed009d0/> enable children properly
 - <csr-id-78d9056e35deb6e87ea2929cc5369e35b8a20847/> it works with a new bump each time!!
 - <csr-id-96b3d56e09a38843f7dc6c1047fe2a3923bd8473/> move back to combined cx/props
 - <csr-id-19df1bda109aba03c40ff631263bcb7035004ca0/> bubbling
 - <csr-id-8acdd2ea830b995b608d8bac2ef527db8d40e662/> it compiles once more
 - <csr-id-74c6211408ea0db5916a4e0973a3f9a7208faa44/> should be functional across the boar
 - <csr-id-55e6dd9701d3710ff17a25581b99b8f159e609b9/> wire tasks up
 - <csr-id-c10c1f418bf21e0e54b53e2babeb37c672c06901/> wire up linked nodes
 - <csr-id-9d8c5ca5ab5784b3f17d7ee20a451ee68fd703d6/> it properly bubbles
 - <csr-id-fd93ee89c19b085a04307ef30217170518defa8e/> upgrade syntax
 - <csr-id-11757ddf61e1decb1bd1c2bb30455d0bd01a3e95/> fake bubbling
 - <csr-id-f2234068ba7cd915a00a81e41660d7d6ee1177cc/> events bubble now
 - <csr-id-2cf90b6903411e42f01a801f89037686194ee068/> pull children out of component definition
 - <csr-id-84fd0c616252bf29cd665782258530032b54d13a/> cleanuup
 - <csr-id-79503f15c5db04fa04575c8735941a2e3a75030b/> full html support
 - <csr-id-6b2645fd80ab20153e7bb51887443e994fbab5cb/> improve FC handling to allow lifetimes of parent
 - <csr-id-fda2ebc2a22965845e015384f39f34ce7cb3e428/> improve safety
 - <csr-id-9726a065b0d4fb1ede5b53a2ddd58c855e51539f/> massage lifetimes
 - <csr-id-efd0e9b5648c809057f339083ba9d454f810d483/> support desktop more completely
 - <csr-id-a2b0c50a343005c63c7032bcefb8323b78350bb9/> add update functionality to useref
 - <csr-id-5502429626023d0788cca352e94ac6ea67c2cb11/> desktop functioning well
 - <csr-id-4a4c7afca7e1beadd4b213214074fdb420eb0923/> shared state mechanisms
 - <csr-id-fac42339c272b0e430ebf4f31b6061a0635d3e19/> mutations
 - <csr-id-687cda1b6d9595357d1dc8720ebe921f61098d8f/> re-enable suspense
 - <csr-id-4a72b3140bd244da602deada1eeecded65ff5848/> amazingly awesome error handling
 - <csr-id-d7940aa2ac2017316d62e0f2eac0701dc6ad1f09/> proper handling of events
 - <csr-id-d717c22d9c5da7b6f343fede11faaf953a3a29e0/> keyed diffing!!
 - <csr-id-fac2e56ed63fc708a89e14df15f1fc32f58391a9/> update root props
 - <csr-id-c321532a6cef40b2d2e4adc8c7a55931b6755b08/> some docs, cleaning
 - <csr-id-1749eba8eb6c8ffa38ab3ad52160b637fe021e86/> more and more sophisticated diff
 - <csr-id-d618092e9d150589e61516f7bbb169f2db49d3f2/> a new vnode type for anchors
 - <csr-id-00231adfa2e1d67a9d7ae2fa61c33e3a22d51978/> code quality improvements for core
 - <csr-id-a2c7d17b0595769f60bc1c2bbf7cbe32cec37486/> mvoe away from compound context
 - <csr-id-31702dbf878dd0207d101f7869ebefd2bb9f6860/> wire up resource pool
 - <csr-id-e5c88fe3a49649ecb308decd14c2557963978619/> make hooks free-functions
 - <csr-id-bfdcb20437c385116fd640e46b6361d09d8d5ff8/> transition to cx, props
 - <csr-id-f782e142118fb7acf1b88a0f3fbb03e4a5e3e91e/> omg what a dumb mistake
 - <csr-id-4837d8e741343d26f31b55e4478a374dc761e538/> suspense!
 - <csr-id-4a0068f09918adbc299150edcf777f342ced0dd3/> bless up, no more segfaults
 - <csr-id-de9f61bcf48c0d6e35e46c337b72a713c9f9f7d2/> more suspended nodes!
 - <csr-id-7dfe89c9581f45a445f17f9fe4bb94e61f67e971/> wire up event delegator for webview
 - <csr-id-0a907b35c647b67172080023462770b761364d51/> fix some lifetime issues
 - <csr-id-80e6c256980eb3e8c32e30f3dbb43c8b3b9a9cf4/> move over to push based mechanism
 - <csr-id-e7238762ae518c5688f9339d11832d17f99ad553/> architecture document and edit list
 - <csr-id-3a57b942624afb8aa6650aeee05466c3c9ce967e/> task system works
   but I broke the other things :(
 - <csr-id-f457b7113129479cad577237ef21cb735fffe483/> rebuild doesn't return errors
 - <csr-id-4091846934b4b3b2bc03d3ca8aaf7712aebd4e36/> add aria
 - <csr-id-c79d9ae674e235c8e9c2c069d24902122b9c7464/> buff up html allowed attributes
 - <csr-id-bbcb5a0234dbce48ffeb64903c3ec04562a87ad6/> enable components in ssr
 - <csr-id-0479252a5fba96e554dcf2422a8566e248fc6593/> keyed diffing
 - <csr-id-9abb0470b7869019d539a2fc21da3872348ae38b/> static node infrastructure and ssr changes
 - <csr-id-e6f56563bc84516e017b3db06f11fab2549b9a50/> tests and benchmarks
 - <csr-id-db6d0184aa5aa612bd7229a06f72b4c2ec5f6409/> dedicated mutations module
 - <csr-id-1cc1679a6ba33ff68d45ad1e964b73230224bc23/> refactor out the hooks implementation
 - <csr-id-2ce0752a9cd253fee1c8b205400aa191b09c9dcb/> fix compiling
 - <csr-id-99d94b69aba192be7e41ebe891aca4bb4f6cae88/> move webview to wry
 - <csr-id-e4cdb645aad800484b19ec35ba1f8bb9ccf71d12/> beaf up the use_state hook
 - <csr-id-22e659c2bd7797ca5a822180aca0cb5d950c5287/> namespaced attributes
   this commit adds namespaced attributes. This lets us support attribute groups, and thus, inline styles.
   
   This namespaced attribute stuff is only available for styles at the moment, though it theoretically could be enabled for any other attributes.
 - <csr-id-904b26f7111c3fc66400744ff6192e4b20bf6d74/> add edits back! and more webview support!
   This commit adds a new type - the DomEdit - for serializing the changes made by the diffing machine. The architecture of how DomEdits fit into the cooperative scheduling is still TBD but it will allow us to build change lists without applying them immediately. This is more performant  and allows us to only render parts of the page at a time.
   
   This commit also adds more infrastructure around webview. Dioxus can now run on the web, generate static pages, run in the desktop, and run on mobile, with a large part of thanks to webview.
 - <csr-id-b5e5ef171aa9f8986fb4ab04d793eb63f557c4ae/> two calculator examples
 - <csr-id-7665f2c6cf05cea64bb9131381d4ac11cbdeb932/> move to slotmap
 - <csr-id-f4fb5bb454536d9f108c7e276ce98a8924ab45e1/> integrate serialization and string borrowing
   This commit adds lifetimes to the diff and realdom methods so consumers may borrow the contents of the DOM for serialization or asynchronous modifications.
 - <csr-id-9d7ee79826a3b3fb952a70abcbb16dcd3363d2fb/> events work again!
 - <csr-id-73047fe95678d50fcfd62a4ace7c6b406c5304e1/> props memoization is more powerful
   This commit solves the memoization , properly memoizing properties that don't have any generic parameters. This is a rough heuristic to prevent non-static lifetimes from creeping into props and breaking our minual lifetime management.
   
   Props that have a generic parameter are opted-out of the `partialeq` requirement and props *without* lifetimes must implement partialeq. We're going to leave manual disabling of memoization for future work.
 - <csr-id-cfa0927cdd40bc3dba22996018605dbad91d0391/> todomvc
 - <csr-id-d4f1ceaffbc0551ea3b179a101885275690cebec/> somewhat working with rc and weak
 - <csr-id-89f22906926ccb5aa4cbe3fe1b6d834b06dd91e7/> dyn scope
 - <csr-id-8329268d39238f030dd703ab4614253e58339e0b/> update builder

### Bug Fixes

 - <csr-id-c439b0ac7e09f70a04262b7c29938d8c52197b76/> component pass thru events
 - <csr-id-4aadec1e30e5e4aa86bdfa56d8fbff9dc7fa1c69/> ci and bug in setter
 - <csr-id-2481cd05c2371b4a23b03d7710598708b1b2e491/> attempt to fix ice
 - <csr-id-bd341f5571580cdf5e495379b49ca988fd9211c3/> tests
 - <csr-id-75fa7b4aa672a8a10afcd11016a1b80e0e6f0f02/> make tests pass
 - <csr-id-3dc0e59876f5aba88ed26f1bbd692820f239d4b0/> readme and examples syntax
 - <csr-id-52c7154897111b570918127ffe3285bb1d5951a0/> really big bug around hooks
 - <csr-id-868f6739d2b2c5f2ace0c5240cff8008901e818c/> keyword length
 - <csr-id-a33f7701fcf5f917fea8719253650b5ad92554fd/> tags
 - <csr-id-601078f9cf78a58d7502a377676ac94f3cf037bf/> desktop and mobile
 - <csr-id-77686ba329cba0679d0339589a00df77d3c4c1a1/> it compiles again
 - <csr-id-27d891934a70424b45e6278b7e2baaa2d1b78b35/> use annotation method from rust/58052 to fix closure lifetimes
 - <csr-id-ba9e1dbb8fa24048a6c9ccef8a8722688226a845/> messed up how lifetimes worked, need to render once per component
 - <csr-id-478255f40d4de1d2e3f3cc9b6d758b30ff394b39/> all the bugs!

### Performance

 - <csr-id-8b3ac0b57ca073c1451e8d5df93882c9360ca52a/> remove global allocation for props
 - <csr-id-ea91fc984dc648150acbafd0abb79d9f42aca500/> refcell to cell for hookidx

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 438 commits contributed to the release over the course of 358 calendar days.
 - 416 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release dioxus-core v0.1.7 ([`16d73b2`](https://github.comgit//DioxusLabs/dioxus/commit/16d73b240fc446c6f6996c19ccf52bbcda2eca79))
    - add title to doc comment ([`d11f322`](https://github.comgit//DioxusLabs/dioxus/commit/d11f322f554e7dbf43b988c9cfda56498cc49872))
    - Release dioxus-core v0.1.7, dioxus-core-macro v0.1.6, dioxus-html v0.1.4, dioxus-desktop v0.1.5, dioxus-hooks v0.1.6, dioxus-mobile v0.0.3, dioxus-router v0.1.0, dioxus-ssr v0.1.2, dioxus-web v0.0.4, dioxus v0.1.7 ([`40d1f85`](https://github.comgit//DioxusLabs/dioxus/commit/40d1f85d0c3e2c9fd23c08840cca9f459d4e4307))
    - better document the `EventHandler` type ([`be9f1a5`](https://github.comgit//DioxusLabs/dioxus/commit/be9f1a52ad2b04f101397ae34482ea7394df653b))
    - component pass thru events ([`c439b0a`](https://github.comgit//DioxusLabs/dioxus/commit/c439b0ac7e09f70a04262b7c29938d8c52197b76))
    - Merge pull request #74 from mrxiaozhuox/master ([`47056fd`](https://github.comgit//DioxusLabs/dioxus/commit/47056fda4577bcbdaa2a6f63d82eec876e5a5aee))
    - Merge pull request #80 from DioxusLabs/jk/router2dotoh ([`cdc2d8e`](https://github.comgit//DioxusLabs/dioxus/commit/cdc2d8ec6d123245c2ea5f6d10af02b6a6833994))
    - ci and bug in setter ([`4aadec1`](https://github.comgit//DioxusLabs/dioxus/commit/4aadec1e30e5e4aa86bdfa56d8fbff9dc7fa1c69))
    - add prevent default attribute and upgrade router ([`427b126`](https://github.comgit//DioxusLabs/dioxus/commit/427b126bc17336d5d14d56eb7fddb8e07752495f))
    - memoize dom in the prescence of identical components ([`cb2782b`](https://github.comgit//DioxusLabs/dioxus/commit/cb2782b4bb34cdaadfff590bfee930ae3ac6536c))
    - make hydration more robust ([`bbb6ee1`](https://github.comgit//DioxusLabs/dioxus/commit/bbb6ee10de824f2e3259576ac01768640c884279))
    - bump all versions ([`4f92ba4`](https://github.comgit//DioxusLabs/dioxus/commit/4f92ba41602d706449c1bddabd49829873ee72eb))
    - attempt to fix ice ([`2481cd0`](https://github.comgit//DioxusLabs/dioxus/commit/2481cd05c2371b4a23b03d7710598708b1b2e491))
    - tests ([`bd341f5`](https://github.comgit//DioxusLabs/dioxus/commit/bd341f5571580cdf5e495379b49ca988fd9211c3))
    - update core, core-macro, and html ([`f9b9bb9`](https://github.comgit//DioxusLabs/dioxus/commit/f9b9bb9c0c2c55f55d2d6860e3d2d986debd6412))
    - eanble bubbling ([`06276ed`](https://github.comgit//DioxusLabs/dioxus/commit/06276edd0d4f1f6f6b0a3bf7a467931413ab33c3))
    - plug in bubbling ([`d84fc05`](https://github.comgit//DioxusLabs/dioxus/commit/d84fc0538670b2a3bda9ae41878896793b74e8ee))
    - run cargo fmt ([`a95dead`](https://github.comgit//DioxusLabs/dioxus/commit/a95dead76d67646a862bb13421c2853cac7604ee))
    - make tests pass ([`75fa7b4`](https://github.comgit//DioxusLabs/dioxus/commit/75fa7b4aa672a8a10afcd11016a1b80e0e6f0f02))
    - remove runner on hook and then update docs ([`d156045`](https://github.comgit//DioxusLabs/dioxus/commit/d1560450bac55f9566e00e00ea405bd1c70b57e5))
    - arbitrary expressions excepted without braces ([`4c85bcf`](https://github.comgit//DioxusLabs/dioxus/commit/4c85bcfdc84184b4fd0fb9317ba31fe569884890))
    - polish some more things ([`1496102`](https://github.comgit//DioxusLabs/dioxus/commit/14961023f927b3a8bde83cfc7883aa8bfcca9e85))
    - bump core version ([`ddfa2ba`](https://github.comgit//DioxusLabs/dioxus/commit/ddfa2bac3f598523e8d5508e7d99a7ff6fa11726))
    - more API updates ([`a4f280d`](https://github.comgit//DioxusLabs/dioxus/commit/a4f280d16399205c638033bf9beb858e478e98ff))
    - clean up examples and demo list ([`944b3a8`](https://github.comgit//DioxusLabs/dioxus/commit/944b3a8bc565da3142ebc8031f9c5fc565877bd8))
    - upgrade hooks ([`b3ac2ee`](https://github.comgit//DioxusLabs/dioxus/commit/b3ac2ee3f76549cd1c7b6f9eee7e3382b07d873c))
    - readme and examples syntax ([`3dc0e59`](https://github.comgit//DioxusLabs/dioxus/commit/3dc0e59876f5aba88ed26f1bbd692820f239d4b0))
    - rip out unsafe task engine ([`c7d001c`](https://github.comgit//DioxusLabs/dioxus/commit/c7d001cbb457929b9742ad96c4997cdcc695bb1a))
    - continue to consolidate ([`21e00c1`](https://github.comgit//DioxusLabs/dioxus/commit/21e00c114ea30ffd5d98d067103fe110b9c53c44))
    - upgrade to new version of dioxus core. ([`cda759c`](https://github.comgit//DioxusLabs/dioxus/commit/cda759c659dfc4b1dde17e3896c35525005026df))
    - clean it up a bit ([`fa106be`](https://github.comgit//DioxusLabs/dioxus/commit/fa106be1f5a45fa5707e66542e52c9f09e8cea7a))
    - enable children properly ([`b997b8e`](https://github.comgit//DioxusLabs/dioxus/commit/b997b8ebbb82b5b9e9119bd2eb25335e2ed009d0))
    - fix ssr ([`ded9696`](https://github.comgit//DioxusLabs/dioxus/commit/ded9696930ec825e0aba990494790e8be43a73e5))
    - remove portals completely ([`2fd56e7`](https://github.comgit//DioxusLabs/dioxus/commit/2fd56e76192bc70d5503bfcd6b4127d383dd082c))
    - some basic cleaning ([`f746793`](https://github.comgit//DioxusLabs/dioxus/commit/f746793890d091d3bca4b7c5261b8b2e2d080512))
    - it works with a new bump each time!! ([`78d9056`](https://github.comgit//DioxusLabs/dioxus/commit/78d9056e35deb6e87ea2929cc5369e35b8a20847))
    - miri stress tets ([`934de21`](https://github.comgit//DioxusLabs/dioxus/commit/934de21dd673b1b79904a3249998427f11428426))
    - go back to noisy lifetime solution ([`8daf7a6`](https://github.comgit//DioxusLabs/dioxus/commit/8daf7a6ed86df72522b089aa2647eea7bee0f3b6))
    - clean up the core crate ([`e6c6bbd`](https://github.comgit//DioxusLabs/dioxus/commit/e6c6bbdc1ec6a8c251b78c05ca104f006b6fad26))
    - adjust memoization ([`e2e4d43`](https://github.comgit//DioxusLabs/dioxus/commit/e2e4d431e14e9e91b3301e994363c042400e687e))
    - adjust semantics of placeholders and fragments ([`1c516ab`](https://github.comgit//DioxusLabs/dioxus/commit/1c516aba6a5dbe11c7a7162f3d65625e62461f03))
    - rename fc to component ([`1e4a599`](https://github.comgit//DioxusLabs/dioxus/commit/1e4a599d14af85a2d1c29a442dd489f8dc8df321))
    - docs ([`8814977`](https://github.comgit//DioxusLabs/dioxus/commit/8814977eeebe06748a3b9677a8070e42a037ebd7))
    - polish ([`8bf57dc`](https://github.comgit//DioxusLabs/dioxus/commit/8bf57dc21dfbcbae5b95650203b68d3f41227652))
    - prepare to change our fragment pattern. Add some more docs ([`2c3a046`](https://github.comgit//DioxusLabs/dioxus/commit/2c3a0464264fa11e8100df025d863931f9606cdb))
    - update hooks ([`597a045`](https://github.comgit//DioxusLabs/dioxus/commit/597a0456f59872bd5dc60d382acdec76a98b1db2))
    - really big bug around hooks ([`52c7154`](https://github.comgit//DioxusLabs/dioxus/commit/52c7154897111b570918127ffe3285bb1d5951a0))
    - better desktop support ([`25a8411`](https://github.comgit//DioxusLabs/dioxus/commit/25a8411485e85bb7e3c8f20701d484529efe9a80))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.1 ([`2b92837`](https://github.comgit//DioxusLabs/dioxus/commit/2b928372fb1b74a4d4e220ff3d798bb7e52f79d2))
    - move back to combined cx/props ([`96b3d56`](https://github.comgit//DioxusLabs/dioxus/commit/96b3d56e09a38843f7dc6c1047fe2a3923bd8473))
    - rename ([`36d89be`](https://github.comgit//DioxusLabs/dioxus/commit/36d89beb34821694cb0afb546d3b0cb4e01aaae1))
    - bubbling ([`19df1bd`](https://github.comgit//DioxusLabs/dioxus/commit/19df1bda109aba03c40ff631263bcb7035004ca0))
    - it compiles once more ([`8acdd2e`](https://github.comgit//DioxusLabs/dioxus/commit/8acdd2ea830b995b608d8bac2ef527db8d40e662))
    - some docs and suspense ([`93d4b8c`](https://github.comgit//DioxusLabs/dioxus/commit/93d4b8ca7c1b133e5dba2a8dc9a310dbe1357001))
    - update readme ([`9bd56ee`](https://github.comgit//DioxusLabs/dioxus/commit/9bd56ee499dd8d3ce2b382655487d150e5a6cacf))
    - should be functional across the boar ([`74c6211`](https://github.comgit//DioxusLabs/dioxus/commit/74c6211408ea0db5916a4e0973a3f9a7208faa44))
    - move examples around ([`1e6e5e6`](https://github.comgit//DioxusLabs/dioxus/commit/1e6e5e611b61571f272289adefc9cdd7d59c4399))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`0d480a4`](https://github.comgit//DioxusLabs/dioxus/commit/0d480a4c437d424f0eaff486e510a8fd3f3e6584))
    - updates to router ([`bab21a0`](https://github.comgit//DioxusLabs/dioxus/commit/bab21a0aa1cbf8e6bd95f823e49f53c082e8d6cc))
    - wire tasks up ([`55e6dd9`](https://github.comgit//DioxusLabs/dioxus/commit/55e6dd9701d3710ff17a25581b99b8f159e609b9))
    - wire up linked nodes ([`c10c1f4`](https://github.comgit//DioxusLabs/dioxus/commit/c10c1f418bf21e0e54b53e2babeb37c672c06901))
    - add router ([`d298b62`](https://github.comgit//DioxusLabs/dioxus/commit/d298b626d3ae21a39a8ec4426373369ac94edf9f))
    - keyword length ([`868f673`](https://github.comgit//DioxusLabs/dioxus/commit/868f6739d2b2c5f2ace0c5240cff8008901e818c))
    - docs and router ([`a5f05d7`](https://github.comgit//DioxusLabs/dioxus/commit/a5f05d73acc0e47b05cff64a373482519414bc7c))
    - it properly bubbles ([`9d8c5ca`](https://github.comgit//DioxusLabs/dioxus/commit/9d8c5ca5ab5784b3f17d7ee20a451ee68fd703d6))
    - upgrade syntax ([`fd93ee8`](https://github.comgit//DioxusLabs/dioxus/commit/fd93ee89c19b085a04307ef30217170518defa8e))
    - more docs ([`9874f34`](https://github.comgit//DioxusLabs/dioxus/commit/9874f342e2bc1116410142c691545d75b0e0d6fe))
    - strong improvmenets, first major section done ([`54d050c`](https://github.comgit//DioxusLabs/dioxus/commit/54d050cf897131ddc4f53651f1aeedaf03f23f57))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`b32665d`](https://github.comgit//DioxusLabs/dioxus/commit/b32665d7212a5b9a3e21cb7af7abba63ae399fac))
    - fake bubbling ([`11757dd`](https://github.comgit//DioxusLabs/dioxus/commit/11757ddf61e1decb1bd1c2bb30455d0bd01a3e95))
    - remove scopechildren in favor of elements directly ([`574d7fd`](https://github.comgit//DioxusLabs/dioxus/commit/574d7fdb9e72a5254130589f13f9efd31e3d2870))
    - remove send requirement on root props ([`5a21493`](https://github.comgit//DioxusLabs/dioxus/commit/5a21493fb7162287989bd7c0cd1c496cc3143605))
    - tags ([`a33f770`](https://github.comgit//DioxusLabs/dioxus/commit/a33f7701fcf5f917fea8719253650b5ad92554fd))
    - events bubble now ([`f223406`](https://github.comgit//DioxusLabs/dioxus/commit/f2234068ba7cd915a00a81e41660d7d6ee1177cc))
    - move hooklist into scope ([`b9fc5fc`](https://github.comgit//DioxusLabs/dioxus/commit/b9fc5fc251b874fe4a9b5bc46718093029c07bad))
    - Merge branch 'master' into jk/remove_node_safety ([`db00047`](https://github.comgit//DioxusLabs/dioxus/commit/db0004758c77331cc3b93ea8cf227c060028e12e))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`3a706ac`](https://github.comgit//DioxusLabs/dioxus/commit/3a706ac4168db137723bea90d7a0058190adfc3c))
    - bubbling reserves nodes ([`b6262ed`](https://github.comgit//DioxusLabs/dioxus/commit/b6262edd53767bf0d87ed1af3bb0c32612380a23))
    - move testdom methods into virtualdom ([`d2f0547`](https://github.comgit//DioxusLabs/dioxus/commit/d2f0547692cb8b4594fbec1d9fa0934e169d1706))
    - pull children out of component definition ([`2cf90b6`](https://github.comgit//DioxusLabs/dioxus/commit/2cf90b6903411e42f01a801f89037686194ee068))
    - Merge branch 'master' of https://github.com/jkelleyrtp/dioxus ([`60d6eb2`](https://github.comgit//DioxusLabs/dioxus/commit/60d6eb204a10633e5e52f91e855bd12c5cda40f2))
    - Various typos/grammar/rewording ([`5747e00`](https://github.comgit//DioxusLabs/dioxus/commit/5747e00b27b1b69c4f9c2820e7e78030feaff71e))
    - update cargo tomls ([`e4c06ce`](https://github.comgit//DioxusLabs/dioxus/commit/e4c06ce8e893779d2aad0883a1bb27d193bc5985))
    - bubbling in progress ([`a21020e`](https://github.comgit//DioxusLabs/dioxus/commit/a21020ea575e467ba0d608737269fe1b0792dba7))
    - cleanup src ([`f2e343c`](https://github.comgit//DioxusLabs/dioxus/commit/f2e343c1548588ad01ddac6e39e5014132cff91c))
    - move children onto scope ([`f438bbc`](https://github.comgit//DioxusLabs/dioxus/commit/f438bbcfd20be889aac9a9ca8486e5896af1ce14))
    - desktop and mobile ([`601078f`](https://github.comgit//DioxusLabs/dioxus/commit/601078f9cf78a58d7502a377676ac94f3cf037bf))
    - start on components ([`9b5f82a`](https://github.comgit//DioxusLabs/dioxus/commit/9b5f82af7dce18b75a781b86b188bc713860bb99))
    - cleanuup ([`84fd0c6`](https://github.comgit//DioxusLabs/dioxus/commit/84fd0c616252bf29cd665782258530032b54d13a))
    - Release dioxus-core v0.1.3, dioxus-core-macro v0.1.2, dioxus-html v0.1.0, dioxus-desktop v0.0.0, dioxus-hooks v0.1.3, dioxus-liveview v0.1.0, dioxus-mobile v0.0.0, dioxus-router v0.1.0, dioxus-ssr v0.1.0, dioxus-web v0.0.0, dioxus v0.1.0 ([`270dfc9`](https://github.comgit//DioxusLabs/dioxus/commit/270dfc9590b2354d083ea8da5cc0e1a1497d30e0))
    - it compiles again ([`77686ba`](https://github.comgit//DioxusLabs/dioxus/commit/77686ba329cba0679d0339589a00df77d3c4c1a1))
    - move debugging virtualdom into its own feature ([`d1b294f`](https://github.comgit//DioxusLabs/dioxus/commit/d1b294fff0c8618e50969ca59f198baf0565777d))
    - clean up using new children syntax ([`9c13436`](https://github.comgit//DioxusLabs/dioxus/commit/9c1343610b3907bd8c6acadc086bfc9a8c7b3f3e))
    - fix some bugs around the rsx macro ([`339e450`](https://github.comgit//DioxusLabs/dioxus/commit/339e450027b4a5d2e1317e13863cd1b2e7ab5853))
    - full html support ([`79503f1`](https://github.comgit//DioxusLabs/dioxus/commit/79503f15c5db04fa04575c8735941a2e3a75030b))
    - slim deps and upgrade docs ([`83dd49d`](https://github.comgit//DioxusLabs/dioxus/commit/83dd49d89044b618fa371e09f9f34ea28b9fd529))
    - work on scheduler, async, coroutines, and merge scope into context ([`b56ea6c`](https://github.comgit//DioxusLabs/dioxus/commit/b56ea6c9a99cd747f376aca4a3c439c13f082714))
    - implement dst magic ([`d4192d0`](https://github.comgit//DioxusLabs/dioxus/commit/d4192d0de2fb352d31f33f2fadfc21de0b1b950c))
    - update local examples and docs to support new syntaxes ([`4de16c4`](https://github.comgit//DioxusLabs/dioxus/commit/4de16c4779648e591b3869b5df31271ae603c812))
    - working on re-enabling components ([`fffc7ea`](https://github.comgit//DioxusLabs/dioxus/commit/fffc7ea061922120cd2288982a310a87691948cd))
    - a few things left, slme cleanup ([`289d2f2`](https://github.comgit//DioxusLabs/dioxus/commit/289d2f2518cef937a6fc823495a8bd32f287b84e))
    - implement lazy nodes properly ([`3e07214`](https://github.comgit//DioxusLabs/dioxus/commit/3e07214272f20a47b168d6431fcb39880cd41a9d))
    - fix table ([`e495b09`](https://github.comgit//DioxusLabs/dioxus/commit/e495b09bf1c99950b5ea8647b371f296dc64afbe))
    - fix rebuild ([`15c31d5`](https://github.comgit//DioxusLabs/dioxus/commit/15c31d545a84f05fe17254009dc275aac93e7574))
    - dst drops properly ([`f33510b`](https://github.comgit//DioxusLabs/dioxus/commit/f33510b13fa515e496e6a934650c5a7c61838a39))
    - remove cx.children. start to move towards a "children" field ([`7b97e00`](https://github.comgit//DioxusLabs/dioxus/commit/7b97e009ec74cd2f134bbb8e576404e7a964db74))
    - clean up, use old approach to components ([`2559740`](https://github.comgit//DioxusLabs/dioxus/commit/2559740463d298d27a058607c5d80196dc736c11))
    - dst nodes leak but work ([`62ed920`](https://github.comgit//DioxusLabs/dioxus/commit/62ed9208a44c7aa038613f73ff0573bd2010f4d0))
    - improve FC handling to allow lifetimes of parent ([`6b2645f`](https://github.comgit//DioxusLabs/dioxus/commit/6b2645fd80ab20153e7bb51887443e994fbab5cb))
    - improve safety ([`fda2ebc`](https://github.comgit//DioxusLabs/dioxus/commit/fda2ebc2a22965845e015384f39f34ce7cb3e428))
    - move more stuff out ([`464b457`](https://github.comgit//DioxusLabs/dioxus/commit/464b457b80a40c0ff24386342a0e81ecfeac4723))
    - try to get lifetimes passed in properly ([`f70b5b4`](https://github.comgit//DioxusLabs/dioxus/commit/f70b5b4e4acf589372f769153968922d2fcfedf1))
    - listen to clippy ([`f492443`](https://github.comgit//DioxusLabs/dioxus/commit/f49244344c06a6d05edfe041fd3afa59ef89ca21))
    - remove bumpframe ([`e4cda7c`](https://github.comgit//DioxusLabs/dioxus/commit/e4cda7c2cbba536b045fa7bed1263b78ed33c096))
    - use annotation method from rust/58052 to fix closure lifetimes ([`27d8919`](https://github.comgit//DioxusLabs/dioxus/commit/27d891934a70424b45e6278b7e2baaa2d1b78b35))
    - it compiles cleanly ([`fd0c384`](https://github.comgit//DioxusLabs/dioxus/commit/fd0c384daccf89cec408e3ad674ea04c00797eae))
    - worked backwards a bit and got it slightly figured out ([`9ee2bfb`](https://github.comgit//DioxusLabs/dioxus/commit/9ee2bfb010ce90ec97e93e173c31aab281db32c4))
    - massage lifetimes ([`9726a06`](https://github.comgit//DioxusLabs/dioxus/commit/9726a065b0d4fb1ede5b53a2ddd58c855e51539f))
    - book documentation ([`16dbf4a`](https://github.comgit//DioxusLabs/dioxus/commit/16dbf4a6f84103857385fb4b142a718b0ce72118))
    - more changes to scheduler ([`059294a`](https://github.comgit//DioxusLabs/dioxus/commit/059294ab55e9e945c9aede1fd4b4faf39a7b9ea9))
    - messed up how lifetimes worked, need to render once per component ([`ba9e1db`](https://github.comgit//DioxusLabs/dioxus/commit/ba9e1dbb8fa24048a6c9ccef8a8722688226a845))
    - major cleanups to scheduler ([`2933e4b`](https://github.comgit//DioxusLabs/dioxus/commit/2933e4bc11b3074c2bde8d76ec55364fca841988))
    - move everything over to a stack dst ([`0e9d5fc`](https://github.comgit//DioxusLabs/dioxus/commit/0e9d5fc5306ab508d5af6999a4064f9b8b48460f))
    - event system ([`1f22a06`](https://github.comgit//DioxusLabs/dioxus/commit/1f22a06a36f72073188b8c9009dd4950b3f4ff9a))
    - no more lifetimes ([`d96276a`](https://github.comgit//DioxusLabs/dioxus/commit/d96276a715a9f62943909cfc429c9fe47fd28146))
    - support desktop more completely ([`efd0e9b`](https://github.comgit//DioxusLabs/dioxus/commit/efd0e9b5648c809057f339083ba9d454f810d483))
    - compiling again with runtime safety ([`857c92f`](https://github.comgit//DioxusLabs/dioxus/commit/857c92f7f096108317b1854715b4dcbeaae6f759))
    - add update functionality to useref ([`a2b0c50`](https://github.comgit//DioxusLabs/dioxus/commit/a2b0c50a343005c63c7032bcefb8323b78350bb9))
    - more raw ptrs ([`95bd17e`](https://github.comgit//DioxusLabs/dioxus/commit/95bd17e38fc936dcc9383d0ba8beac5ed64b41eb))
    - lnks to projects ([`460783a`](https://github.comgit//DioxusLabs/dioxus/commit/460783ad549818a85db634ed9c39ffce210b98ec))
    - desktop functioning well ([`5502429`](https://github.comgit//DioxusLabs/dioxus/commit/5502429626023d0788cca352e94ac6ea67c2cb11))
    - remove resource pool in favor of raw ptrs ([`b8d9869`](https://github.comgit//DioxusLabs/dioxus/commit/b8d98698c6056a330fe62f52833504fb1c728bbb))
    - overhaul event system ([`7a03c1d`](https://github.comgit//DioxusLabs/dioxus/commit/7a03c1d2b48590276b182465679387655fe08f3a))
    - consolidation, simplification ([`70b983d`](https://github.comgit//DioxusLabs/dioxus/commit/70b983d0909a332d357da9bb07adc263059bce19))
    - threadsafe ([`82953f2`](https://github.comgit//DioxusLabs/dioxus/commit/82953f2ac37913f83a822333acd0c47e20777d31))
    - all the bugs! ([`478255f`](https://github.comgit//DioxusLabs/dioxus/commit/478255f40d4de1d2e3f3cc9b6d758b30ff394b39))
    - ssr ([`71f0df6`](https://github.comgit//DioxusLabs/dioxus/commit/71f0df63745fe5c17468693144c552ea3a0a7101))
    - shared state mechanisms ([`4a4c7af`](https://github.comgit//DioxusLabs/dioxus/commit/4a4c7afca7e1beadd4b213214074fdb420eb0923))
    - fix typo ([`4a5041c`](https://github.comgit//DioxusLabs/dioxus/commit/4a5041c5902e1dab99f25ce00d754ac8a5ee30c1))
    - fix web list issue ([`da4423c`](https://github.comgit//DioxusLabs/dioxus/commit/da4423c141f1f376df5f3f2580e5284831744a7e))
    - move macro crate out of core ([`7bdad1e`](https://github.comgit//DioxusLabs/dioxus/commit/7bdad1e2e6f67e74c9f67dde2150140cf8a090e8))
    - portals ([`4a0bb8c`](https://github.comgit//DioxusLabs/dioxus/commit/4a0bb8cf84da7960d7090de3d5f95248318540a3))
    - polish ([`a5f82be`](https://github.comgit//DioxusLabs/dioxus/commit/a5f82be433309790e0cac07a77e59c996a53796d))
    - exampels ([`04d16b4`](https://github.comgit//DioxusLabs/dioxus/commit/04d16b41dc9bc06ca0320a9fec27744318e48355))
    - fix some event stuff for web and core ([`725b4a1`](https://github.comgit//DioxusLabs/dioxus/commit/725b4a1d7f5d629b1b0a163b65bfd93b2f8a151b))
    - garbage collection ([`dfe9f86`](https://github.comgit//DioxusLabs/dioxus/commit/dfe9f86bfff48ae3166fd41d673245ed142c26f0))
    - on collaborative scheduling ([`1a32383`](https://github.comgit//DioxusLabs/dioxus/commit/1a323835c8c4f667e5744bbf5447776f1dc51fca))
    - examples ([`1a2f91e`](https://github.comgit//DioxusLabs/dioxus/commit/1a2f91ed91c13dae553ecde585462ab261b1b95d))
    - working down warnings ([`958c02a`](https://github.comgit//DioxusLabs/dioxus/commit/958c02a26e86b07431d3e5f51f00afe116656981))
    - mutations ([`fac4233`](https://github.comgit//DioxusLabs/dioxus/commit/fac42339c272b0e430ebf4f31b6061a0635d3e19))
    - performance looks good, needs more testing ([`4b6ca05`](https://github.comgit//DioxusLabs/dioxus/commit/4b6ca05f2c3ad647842c858967da9c87f1915825))
    - add test_dom ([`a652090`](https://github.comgit//DioxusLabs/dioxus/commit/a652090dc5708db334fa7430fededb1bac207880))
    - bottom up dropping ([`f2334c1`](https://github.comgit//DioxusLabs/dioxus/commit/f2334c17be2612d926361686d7d40a57e3ffe9b9))
    - cleanup ([`1745a44`](https://github.comgit//DioxusLabs/dioxus/commit/1745a44d949b994b64ea1fb715cbe36963ae7027))
    - more scheduler work ([`44b4384`](https://github.comgit//DioxusLabs/dioxus/commit/44b43849613a52cee8dc98ea3068095416b201c2))
    - re-enable suspense ([`687cda1`](https://github.comgit//DioxusLabs/dioxus/commit/687cda1b6d9595357d1dc8720ebe921f61098d8f))
    - fill out the snippets ([`6051b0e`](https://github.comgit//DioxusLabs/dioxus/commit/6051b0ec86927704451f4ce6cdf8f988e59702ae))
    - amazingly awesome error handling ([`4a72b31`](https://github.comgit//DioxusLabs/dioxus/commit/4a72b3140bd244da602deada1eeecded65ff5848))
    - some ideas ([`05c909f`](https://github.comgit//DioxusLabs/dioxus/commit/05c909f320765aec1bf4c1c55ca59ffd5525a2c7))
    - proper handling of events ([`d7940aa`](https://github.comgit//DioxusLabs/dioxus/commit/d7940aa2ac2017316d62e0f2eac0701dc6ad1f09))
    - more work on scheduler ([`1cd5e69`](https://github.comgit//DioxusLabs/dioxus/commit/1cd5e697128877e911ece2c9b34ec38d74eeec80))
    - websys dom working properly ([`cfa0247`](https://github.comgit//DioxusLabs/dioxus/commit/cfa0247cbb1233e1df275374a73f431650a9250f))
    - big updates to the reference ([`583fdfa`](https://github.comgit//DioxusLabs/dioxus/commit/583fdfa5618e11d660985b97e570d4503be2ff49))
    - fix invalid is_empty bug ([`c7d2d9d`](https://github.comgit//DioxusLabs/dioxus/commit/c7d2d9d050548f06244caeeedc2c0e0e52eb329f))
    - more work on rebuild ([`5c3dc0a`](https://github.comgit//DioxusLabs/dioxus/commit/5c3dc0a8741df251dd3cb7f954fae2243df155f7))
    - keyed diffing!! ([`d717c22`](https://github.comgit//DioxusLabs/dioxus/commit/d717c22d9c5da7b6f343fede11faaf953a3a29e0))
    - docs, html! macro, more ([`caf772c`](https://github.comgit//DioxusLabs/dioxus/commit/caf772cf249d2f56c8d0b0fa2737ad48e32c6e82))
    - update root props ([`fac2e56`](https://github.comgit//DioxusLabs/dioxus/commit/fac2e56ed63fc708a89e14df15f1fc32f58391a9))
    - get keyed diffing compiling ([`0a0be95`](https://github.comgit//DioxusLabs/dioxus/commit/0a0be95c3e58dc065409f02f703b82700c1003f8))
    - changes to scheduler ([`098d382`](https://github.comgit//DioxusLabs/dioxus/commit/098d3821ed89ad38d99077a6556b48a7e91fc3fc))
    - some docs, cleaning ([`c321532`](https://github.comgit//DioxusLabs/dioxus/commit/c321532a6cef40b2d2e4adc8c7a55931b6755b08))
    - more and more sophisticated diff ([`1749eba`](https://github.comgit//DioxusLabs/dioxus/commit/1749eba8eb6c8ffa38ab3ad52160b637fe021e86))
    - clean up warnings ([`b32e261`](https://github.comgit//DioxusLabs/dioxus/commit/b32e2611e37b17c2371ffb10cf1ac647f017d917))
    - a new vnode type for anchors ([`d618092`](https://github.comgit//DioxusLabs/dioxus/commit/d618092e9d150589e61516f7bbb169f2db49d3f2))
    - more cleanup ([`5cbdc57`](https://github.comgit//DioxusLabs/dioxus/commit/5cbdc571e3abe54391aec8f296d7389410b373b6))
    - more work on bumpframe ([`63a85e4`](https://github.comgit//DioxusLabs/dioxus/commit/63a85e4865929dbd32a1b72ebeac271561e9ed92))
    - heuristics engine ([`6aaad1c`](https://github.comgit//DioxusLabs/dioxus/commit/6aaad1c9ef97fa407b660aec60999d263c172ecc))
    - more cleanup in scheduler ([`9f99f46`](https://github.comgit//DioxusLabs/dioxus/commit/9f99f46cfd6355ce46afdf84d60dd049047c6b57))
    - making progress on diffing and hydration ([`49856cc`](https://github.comgit//DioxusLabs/dioxus/commit/49856ccd6865f88d63765f26d27f7e945b554da0))
    - code quality improvements for core ([`00231ad`](https://github.comgit//DioxusLabs/dioxus/commit/00231adfa2e1d67a9d7ae2fa61c33e3a22d51978))
    - mvoe away from compound context ([`a2c7d17`](https://github.comgit//DioxusLabs/dioxus/commit/a2c7d17b0595769f60bc1c2bbf7cbe32cec37486))
    - examples ([`f1cff84`](https://github.comgit//DioxusLabs/dioxus/commit/f1cff845ce11231cb4b2dd4857f9ca9f0265b925))
    - wire up resource pool ([`31702db`](https://github.comgit//DioxusLabs/dioxus/commit/31702dbf878dd0207d101f7869ebefd2bb9f6860))
    - make hooks free-functions ([`e5c88fe`](https://github.comgit//DioxusLabs/dioxus/commit/e5c88fe3a49649ecb308decd14c2557963978619))
    - more work on suspense and documentation ([`37ed4be`](https://github.comgit//DioxusLabs/dioxus/commit/37ed4bed8cf28eb65465d41e15e8d758cb3d9679))
    - transition to cx, props ([`bfdcb20`](https://github.comgit//DioxusLabs/dioxus/commit/bfdcb20437c385116fd640e46b6361d09d8d5ff8))
    - more doc ([`adf202e`](https://github.comgit//DioxusLabs/dioxus/commit/adf202eab9201c69ec455e621c755500329815fe))
    - cleanup public apis ([`927b05f`](https://github.comgit//DioxusLabs/dioxus/commit/927b05f358221aa4b9c440608704e86c2400ab64))
    - omg what a dumb mistake ([`f782e14`](https://github.comgit//DioxusLabs/dioxus/commit/f782e142118fb7acf1b88a0f3fbb03e4a5e3e91e))
    - suspense! ([`4837d8e`](https://github.comgit//DioxusLabs/dioxus/commit/4837d8e741343d26f31b55e4478a374dc761e538))
    - refactor ([`8b0eb87`](https://github.comgit//DioxusLabs/dioxus/commit/8b0eb87c72ea9d444dee99a8b05643f19fea2634))
    - bless up, no more segfaults ([`4a0068f`](https://github.comgit//DioxusLabs/dioxus/commit/4a0068f09918adbc299150edcf777f342ced0dd3))
    - more suspended nodes! ([`de9f61b`](https://github.comgit//DioxusLabs/dioxus/commit/de9f61bcf48c0d6e35e46c337b72a713c9f9f7d2))
    - more docs ([`34c3107`](https://github.comgit//DioxusLabs/dioxus/commit/34c3107418df89620cb9dfbd9300f89a1fe6e68f))
    - algo for scheduler finished ([`9ad5e49`](https://github.comgit//DioxusLabs/dioxus/commit/9ad5e4965409464c6ca63b3434bbe182dde26943))
    - wire up event delegator for webview ([`7dfe89c`](https://github.comgit//DioxusLabs/dioxus/commit/7dfe89c9581f45a445f17f9fe4bb94e61f67e971))
    - basic support for scheduled rendering ([`c52af22`](https://github.comgit//DioxusLabs/dioxus/commit/c52af221f755601a9e826ffc2c355def138999d0))
    - feeling goodish about scheduler ([`5f63eda`](https://github.comgit//DioxusLabs/dioxus/commit/5f63eda29473b82ce708ec66e4dd862c3717e958))
    - fix some lifetime issues ([`0a907b3`](https://github.comgit//DioxusLabs/dioxus/commit/0a907b35c647b67172080023462770b761364d51))
    - move over to push based mechanism ([`80e6c25`](https://github.comgit//DioxusLabs/dioxus/commit/80e6c256980eb3e8c32e30f3dbb43c8b3b9a9cf4))
    - bubbling ([`7978a17`](https://github.comgit//DioxusLabs/dioxus/commit/7978a17419be0c1e6a7ddd2c21cfa96b9660038b))
    - some structure ([`bcaa76a`](https://github.comgit//DioxusLabs/dioxus/commit/bcaa76a37fb7ff48ab3345c594a20fbdcda6074a))
    - solve some issues regarding listeners ([`dfaf5ad`](https://github.comgit//DioxusLabs/dioxus/commit/dfaf5adee164f44a679ab21d730caaab3610e01f))
    - architecture document and edit list ([`e723876`](https://github.comgit//DioxusLabs/dioxus/commit/e7238762ae518c5688f9339d11832d17f99ad553))
    - before scheduler simplify ([`c16b92e`](https://github.comgit//DioxusLabs/dioxus/commit/c16b92ee7ee3a02153d651db1a84c7dc781c26da))
    - considering making vdom passed verywehre ([`ac3a7b1`](https://github.comgit//DioxusLabs/dioxus/commit/ac3a7b1fbc1665482354ddf4f0de3d6ceb5b400a))
    - move to slab ([`6084fbc`](https://github.comgit//DioxusLabs/dioxus/commit/6084fbcd11d1669db135098d920488e2c01a7014))
    - change in cx to cx ([`9971ff2`](https://github.comgit//DioxusLabs/dioxus/commit/9971ff215db6f771b7ec1cae2517c85d47d38622))
    - more work on scheduler ([`d4d7114`](https://github.comgit//DioxusLabs/dioxus/commit/d4d7114bebe09a5e265a29105a9bdf3b369ecd67))
    - move things into a "shared" object ([`f644d7c`](https://github.comgit//DioxusLabs/dioxus/commit/f644d7c44159eef091552dcc90acbb151ea76b21))
    - reduce warnings ([`0ded32e`](https://github.comgit//DioxusLabs/dioxus/commit/0ded32e184a4b48b848b078585fbe1ca61d09b1d))
    - it compiles again ([`07efb6a`](https://github.comgit//DioxusLabs/dioxus/commit/07efb6a1b656b24eb1ff25c62f84bb529df936b2))
    - task system works ([`3a57b94`](https://github.comgit//DioxusLabs/dioxus/commit/3a57b942624afb8aa6650aeee05466c3c9ce967e))
    - more polish on display ([`12afd2b`](https://github.comgit//DioxusLabs/dioxus/commit/12afd2b0f24c6968c1498fcedebb03a06e9bc135))
    - more overhaul on virtualevents ([`41cc429`](https://github.comgit//DioxusLabs/dioxus/commit/41cc42919d42453f8f2560aa852211364af4ad3d))
    - cargo fix ([`84c3e9f`](https://github.comgit//DioxusLabs/dioxus/commit/84c3e9fcb11bab921818fbb090632a54dc16423f))
    - more polish on scopes ([`2386772`](https://github.comgit//DioxusLabs/dioxus/commit/238677208ea93d39e26b17af0c17c4f0b512d007))
    - more work on priority ([`cef116a`](https://github.comgit//DioxusLabs/dioxus/commit/cef116aa3a435857c9a3a818bcd81de8ef03a771))
    - cargo fix ([`beceda5`](https://github.comgit//DioxusLabs/dioxus/commit/beceda511c514d743bce17ed19df9dba8675fcda))
    - polish up some safety stuff and add suspense support in ([`ff1398b`](https://github.comgit//DioxusLabs/dioxus/commit/ff1398b943de14f7573ad13577e83500ed9c146e))
    - refactor to fix some ownership quirks ([`05e960b`](https://github.comgit//DioxusLabs/dioxus/commit/05e960b6b037d237ddcaf4fef4d33179883b6644))
    - cargo fix ([`fd03b9d`](https://github.comgit//DioxusLabs/dioxus/commit/fd03b9d252c9e57a281557fbe91786fba2f28e3b))
    - remove unnecessary import ([`fb0c6d2`](https://github.comgit//DioxusLabs/dioxus/commit/fb0c6d2ea4d1edbf1cec4c78179395114071a520))
    - fix props update ([`79e8df2`](https://github.comgit//DioxusLabs/dioxus/commit/79e8df26f5a84f5d765921ae307b33db83c14d4e))
    - more work on web ([`3bf19d8`](https://github.comgit//DioxusLabs/dioxus/commit/3bf19d8106e0cca7749b7346e9b0adfce654d782))
    - cargo fix ([`c7ca7c2`](https://github.comgit//DioxusLabs/dioxus/commit/c7ca7c227325dee85fec0f1bc7d8ab00cbf68f66))
    - more clean up ([`5a09ec1`](https://github.comgit//DioxusLabs/dioxus/commit/5a09ec161031abaa01582435839a8077fd80b228))
    - ricraf polyfill ([`59219b9`](https://github.comgit//DioxusLabs/dioxus/commit/59219b9ef22caa5896046ccaabc8a225cc4b5a53))
    - cut down on errors ([`775e9e2`](https://github.comgit//DioxusLabs/dioxus/commit/775e9e29b8cde45a13bdae22a6f54b25518e7192))
    - ....sigh..... so the diffing algorithm is robust ([`68ed1c0`](https://github.comgit//DioxusLabs/dioxus/commit/68ed1c04e7e773f9e6c0a5148f0ea89b97b6784e))
    - remove global allocation for props ([`8b3ac0b`](https://github.comgit//DioxusLabs/dioxus/commit/8b3ac0b57ca073c1451e8d5df93882c9360ca52a))
    - ric_raf wired up ([`8b0d04c`](https://github.comgit//DioxusLabs/dioxus/commit/8b0d04ce585596f561f15e92fc087e2c7063eb07))
    - wip ([`996247a`](https://github.comgit//DioxusLabs/dioxus/commit/996247a1644d91ebb00e2b2188d21cacdc48257b))
    - lots of changes to diffing ([`ff0a3d1`](https://github.comgit//DioxusLabs/dioxus/commit/ff0a3d1c83a4349a228a742fccc761822f1711d3))
    - change around how deadline works ([`369b36b`](https://github.comgit//DioxusLabs/dioxus/commit/369b36b2c64a4bc3d4895f1e04b412c4edbf3c99))
    - rebuild doesn't return errors ([`f457b71`](https://github.comgit//DioxusLabs/dioxus/commit/f457b7113129479cad577237ef21cb735fffe483))
    - add aria ([`4091846`](https://github.comgit//DioxusLabs/dioxus/commit/4091846934b4b3b2bc03d3ca8aaf7712aebd4e36))
    - more examples ([`56e7eb8`](https://github.comgit//DioxusLabs/dioxus/commit/56e7eb83a97ebd6d5bcd23464cfb9d718e5ac26d))
    - tests and documentation ([`da81591`](https://github.comgit//DioxusLabs/dioxus/commit/da8159190b84d13caa9dfe7dea385a809cf84199))
    - buff up html allowed attributes ([`c79d9ae`](https://github.comgit//DioxusLabs/dioxus/commit/c79d9ae674e235c8e9c2c069d24902122b9c7464))
    - move examples around ([`304259d`](https://github.comgit//DioxusLabs/dioxus/commit/304259d8186d1d34224a74c95f4fd7d14126b499))
    - it works but the page is backwards ([`cdcd861`](https://github.comgit//DioxusLabs/dioxus/commit/cdcd8611e87ffb5e24de7b9fe6c656af3053276e))
    - use the new structure ([`a05047d`](https://github.comgit//DioxusLabs/dioxus/commit/a05047d01e606425fb0d6595e9d27d3e15f32050))
    - ssr + tide ([`269e81b`](https://github.comgit//DioxusLabs/dioxus/commit/269e81b0fdb32ae0706160cd278cf3a1b731387b))
    - more test ([`a0a977a`](https://github.comgit//DioxusLabs/dioxus/commit/a0a977a12d4008155fe332ee226f04e5f59acf95))
    - enable components in ssr ([`bbcb5a0`](https://github.comgit//DioxusLabs/dioxus/commit/bbcb5a0234dbce48ffeb64903c3ec04562a87ad6))
    - documentation and badges ([`3f68be7`](https://github.comgit//DioxusLabs/dioxus/commit/3f68be79ae326f6d169ee771bf86490aed4d7885))
    - keyed diffing ([`0479252`](https://github.comgit//DioxusLabs/dioxus/commit/0479252a5fba96e554dcf2422a8566e248fc6593))
    - static node infrastructure and ssr changes ([`9abb047`](https://github.comgit//DioxusLabs/dioxus/commit/9abb0470b7869019d539a2fc21da3872348ae38b))
    - naming ([`1a6b238`](https://github.comgit//DioxusLabs/dioxus/commit/1a6b2388299bf005006e23b75c65a09449049f8d))
    - it works? ([`778baff`](https://github.comgit//DioxusLabs/dioxus/commit/778baffb10c2b14ac350b38229d1b3acfa49ec39))
    - more refactor for async ([`975fa56`](https://github.comgit//DioxusLabs/dioxus/commit/975fa566f9809f8fa2bb0bdb07fbfc7f855dcaeb))
    - refactor non_keyed diff ([`082765f`](https://github.comgit//DioxusLabs/dioxus/commit/082765f6873d9ba72d67e3383dfab6c901eb772a))
    - some project refactor ([`8cfc437`](https://github.comgit//DioxusLabs/dioxus/commit/8cfc437bfe4110d7f984428f01df90bdf8f8d9ec))
    - tests and benchmarks ([`e6f5656`](https://github.comgit//DioxusLabs/dioxus/commit/e6f56563bc84516e017b3db06f11fab2549b9a50))
    - some work ([`feab50f`](https://github.comgit//DioxusLabs/dioxus/commit/feab50f24a8ab82a2cf1d02af1c54d13dd539b01))
    - structure coming together and tests ([`2d541ec`](https://github.comgit//DioxusLabs/dioxus/commit/2d541eca640d15bb56bccb4191b1dc0fe02dfc22))
    - more refactor ([`58ab51a`](https://github.comgit//DioxusLabs/dioxus/commit/58ab51a4e4e8a2a7b9b03a4c9a136648112d39dc))
    - compiles again ([`16bfc6d`](https://github.comgit//DioxusLabs/dioxus/commit/16bfc6d2488f7ec0266ff5b174142232fb04849f))
    - more refactor ([`c811a89`](https://github.comgit//DioxusLabs/dioxus/commit/c811a8982c08a778f1484ed8aa1dd6b991790086))
    - dedicated mutations module ([`db6d018`](https://github.comgit//DioxusLabs/dioxus/commit/db6d0184aa5aa612bd7229a06f72b4c2ec5f6409))
    - refactor ([`9276fd7`](https://github.comgit//DioxusLabs/dioxus/commit/9276fd7db78e48c530b8f5dbc317be2d8d51ad7f))
    - basic creates working properly ([`1622f48`](https://github.comgit//DioxusLabs/dioxus/commit/1622f484b34412110723645cffc4c8c286b6ca80))
    - refactor out the hooks implementation ([`1cc1679`](https://github.comgit//DioxusLabs/dioxus/commit/1cc1679a6ba33ff68d45ad1e964b73230224bc23))
    - tests added for new iterative ([`24572b6`](https://github.comgit//DioxusLabs/dioxus/commit/24572b63deec590f810df0b95a12cbd5b0bd61f5))
    - webview and async ([`eb82051`](https://github.comgit//DioxusLabs/dioxus/commit/eb82051000fe8772b97f879195fc412092228023))
    - fix compiling ([`2ce0752`](https://github.comgit//DioxusLabs/dioxus/commit/2ce0752a9cd253fee1c8b205400aa191b09c9dcb))
    - move webview to wry ([`99d94b6`](https://github.comgit//DioxusLabs/dioxus/commit/99d94b69aba192be7e41ebe891aca4bb4f6cae88))
    - moving over instructions ([`0987760`](https://github.comgit//DioxusLabs/dioxus/commit/098776095844013469f68c0b585f75587c0edd75))
    - more examples ([`2547da3`](https://github.comgit//DioxusLabs/dioxus/commit/2547da36a089f09d219e7d701efd35f74f66784c))
    - remove old create ([`7b06820`](https://github.comgit//DioxusLabs/dioxus/commit/7b068202cea3e513cf885826f303df7e4f707afd))
    - beaf up the use_state hook ([`e4cdb64`](https://github.comgit//DioxusLabs/dioxus/commit/e4cdb645aad800484b19ec35ba1f8bb9ccf71d12))
    - back to vnode enum ([`64f289a`](https://github.comgit//DioxusLabs/dioxus/commit/64f289a61c18b6b4c1adf785a864171e51615780))
    - move from recursive to iterative ([`9652ccd`](https://github.comgit//DioxusLabs/dioxus/commit/9652ccdcf19189e2aac9262bcfa30e2b04c36d37))
    - move CLI into its own "studio" app ([`fd79335`](https://github.comgit//DioxusLabs/dioxus/commit/fd7933561fe81922e4d5d77f6ac3b6f19efb5a90))
    - working on async diff ([`f41cff5`](https://github.comgit//DioxusLabs/dioxus/commit/f41cff571fd11bb00820c0bf9d147e81ed5a6a53))
    - more work on scheduler ([`882d695`](https://github.comgit//DioxusLabs/dioxus/commit/882d69571dcc807a5fa3734f14980e5233f895f5))
    - move some examples around ([`98a0933`](https://github.comgit//DioxusLabs/dioxus/commit/98a09339fd3190799ea4dd316908f0a53fdf2413))
    - scheduler skeleton ([`9d14faf`](https://github.comgit//DioxusLabs/dioxus/commit/9d14faf62c654e6958b5e5dc444374146977f52c))
    - remove the scoped trait ([`cca7c5f`](https://github.comgit//DioxusLabs/dioxus/commit/cca7c5fc3a9e341996e2d5a23b981b658e54ea62))
    - more work on deadline, support immediates ([`5965bee`](https://github.comgit//DioxusLabs/dioxus/commit/5965bee1d143ee50ed24d4bde5176e057bce4363))
    - move the functions around a bit ([`36542b4`](https://github.comgit//DioxusLabs/dioxus/commit/36542b482f33e5abd58d3d2c40250f6ffac1a99d))
    - got the structure figured out! ([`97745c6`](https://github.comgit//DioxusLabs/dioxus/commit/97745c6a7f2094deca32c7a1c30890853067848e))
    - fix issues with lifetimes ([`a38a81e`](https://github.comgit//DioxusLabs/dioxus/commit/a38a81e1290375cae685f7c49d3745e4298fab26))
    - close on putting it all together ([`85e2dc2`](https://github.comgit//DioxusLabs/dioxus/commit/85e2dc259aba38ec1654ab7581418aaa9c1c543c))
    - namespaced attributes ([`22e659c`](https://github.comgit//DioxusLabs/dioxus/commit/22e659c2bd7797ca5a822180aca0cb5d950c5287))
    - groundwork for noderefs ([`c1afeba`](https://github.comgit//DioxusLabs/dioxus/commit/c1afeba1efb1a063705466a14648beee08cacb86))
    - more work on jank free ([`a44e9fc`](https://github.comgit//DioxusLabs/dioxus/commit/a44e9fcffa4da6dd01966a62575ff4f4832e5fcc))
    - more examples ([`11f89e5`](https://github.comgit//DioxusLabs/dioxus/commit/11f89e5d338d14a7aeece0a6275c24ae65913ce7))
    - add edits back! and more webview support! ([`904b26f`](https://github.comgit//DioxusLabs/dioxus/commit/904b26f7111c3fc66400744ff6192e4b20bf6d74))
    - enable more diffing ([`e8f29a8`](https://github.comgit//DioxusLabs/dioxus/commit/e8f29a8f8ac56020bee0048021efa52547307a77))
    - two calculator examples ([`b5e5ef1`](https://github.comgit//DioxusLabs/dioxus/commit/b5e5ef171aa9f8986fb4ab04d793eb63f557c4ae))
    - examples ([`d9e6d09`](https://github.comgit//DioxusLabs/dioxus/commit/d9e6d0925b30690212d1d690dfba288f1a694a27))
    - wip ([`952a91d`](https://github.comgit//DioxusLabs/dioxus/commit/952a91d5408aaf789b496f11d01c3b3f7fcf9059))
    - integrate signals ([`93900aa`](https://github.comgit//DioxusLabs/dioxus/commit/93900aac4443db2e63631ed928294af74201a9ff))
    - move to slotmap ([`7665f2c`](https://github.comgit//DioxusLabs/dioxus/commit/7665f2c6cf05cea64bb9131381d4ac11cbdeb932))
    - more work on diffing ([`21685fb`](https://github.comgit//DioxusLabs/dioxus/commit/21685fbb164cf58120e9d37e994a25fa7d902b21))
    - integrate serialization and string borrowing ([`f4fb5bb`](https://github.comgit//DioxusLabs/dioxus/commit/f4fb5bb454536d9f108c7e276ce98a8924ab45e1))
    - more work on diffing machine ([`9813f23`](https://github.comgit//DioxusLabs/dioxus/commit/9813f23cdf7b32bc058771c91db3f182d6224905))
    - more work on diffing. ([`ca3b80f`](https://github.comgit//DioxusLabs/dioxus/commit/ca3b80f06929f30a32c4d50387977cc8724daa8d))
    - stack-based "real child iterator" ([`895cc01`](https://github.comgit//DioxusLabs/dioxus/commit/895cc0142b8c0a380ecc71aa6acb5c9d7820862d))
    - more docs ([`0826fdf`](https://github.comgit//DioxusLabs/dioxus/commit/0826fdfee13d26b08cb8a2fa45cbd59cc4f20c25))
    - refcell to cell for hookidx ([`ea91fc9`](https://github.comgit//DioxusLabs/dioxus/commit/ea91fc984dc648150acbafd0abb79d9f42aca500))
    - rename ctx to cx ([`81382e7`](https://github.comgit//DioxusLabs/dioxus/commit/81382e7044fb3dba61d4abb1e6086b7b29143116))
    - rethinking stack machine ([`62ae5d3`](https://github.comgit//DioxusLabs/dioxus/commit/62ae5d3bb94cb9ead030ae0b39d9d9bc2b8b4532))
    - move around examples ([`70cd46d`](https://github.comgit//DioxusLabs/dioxus/commit/70cd46dbb2a689ae2d512e142b8aee9c80798430))
    - start moving events to rc<event> ([`b9ff95f`](https://github.comgit//DioxusLabs/dioxus/commit/b9ff95fa12c46365fe73b64a4926a506d5da2342))
    - rename recoil to atoms ([`36ea39a`](https://github.comgit//DioxusLabs/dioxus/commit/36ea39ae30aa3f1fb2d718c0fdf08850c6bfd3ac))
    - more examples and docs ([`7fbaf69`](https://github.comgit//DioxusLabs/dioxus/commit/7fbaf69cabbdde712bb3fd9e4b2a5dc18b9390e9))
    - more work on updating syntad ([`47e8960`](https://github.comgit//DioxusLabs/dioxus/commit/47e896038ef3655566f3eda83d1d2adfefbc8862))
    - some cleanup and documentation ([`517d7f1`](https://github.comgit//DioxusLabs/dioxus/commit/517d7f14957c4dae9fc894bfbdcd00a955d09f20))
    - pre-diffmachine merge fork ([`424a181`](https://github.comgit//DioxusLabs/dioxus/commit/424a18137f1fa9528408ce96a57e8120251b9ed5))
    - docs ([`f5683a2`](https://github.comgit//DioxusLabs/dioxus/commit/f5683a23464992ecace463a61414795b5a2c58c8))
    - pre vnodes instead of vnode ([`fe6938c`](https://github.comgit//DioxusLabs/dioxus/commit/fe6938ceb3dba0796ae8bab52ae41248dc0d3650))
    - move into a fromjs tutorial ([`69f5cc3`](https://github.comgit//DioxusLabs/dioxus/commit/69f5cc3802af136729bc73e5c3d209270d41b184))
    - events work again! ([`9d7ee79`](https://github.comgit//DioxusLabs/dioxus/commit/9d7ee79826a3b3fb952a70abcbb16dcd3363d2fb))
    - successfully building ([`e3d9db0`](https://github.comgit//DioxusLabs/dioxus/commit/e3d9db0847ecc4ff5156929adb6d22e324f93df3))
    - props memoization is more powerful ([`73047fe`](https://github.comgit//DioxusLabs/dioxus/commit/73047fe95678d50fcfd62a4ace7c6b406c5304e1))
    - merge in some code from the other branch ([`7790750`](https://github.comgit//DioxusLabs/dioxus/commit/7790750349b40055673a0ec16074a0426b84d3b3))
    - move the rsx macro around ([`50c8b93`](https://github.comgit//DioxusLabs/dioxus/commit/50c8b93aade1bfa83a091fb51ee48638507f89b0))
    - add some more sources in the core implementation ([`7102fe5`](https://github.comgit//DioxusLabs/dioxus/commit/7102fe5f984fe8692cf977ea3a43e2973eed4f45))
    - new approach at direct access to vdom ([`795a54a`](https://github.comgit//DioxusLabs/dioxus/commit/795a54a2e427bc7f2b37e636d50aa300977a42a1))
    - some docs ([`1919f88`](https://github.comgit//DioxusLabs/dioxus/commit/1919f88f035fadadfc2b9bce6157f2effcd2a4b0))
    - get diff compiling ([`cff0547`](https://github.comgit//DioxusLabs/dioxus/commit/cff0547f1a3e2e28ba1946f347c161078818ac66))
    - massive changes to definition of components ([`508c560`](https://github.comgit//DioxusLabs/dioxus/commit/508c560320d78730fa058156421523ffa5695d9d))
    - moving to IDs ([`79127ea`](https://github.comgit//DioxusLabs/dioxus/commit/79127ea6cdabf62bf91e777aaacb563e3aa5e619))
    - move to static props ([`c1fd848`](https://github.comgit//DioxusLabs/dioxus/commit/c1fd848f89b0146581d8e485fa0d4a847387b963))
    - moving to imperative method of dom ([`45ee803`](https://github.comgit//DioxusLabs/dioxus/commit/45ee803609bf94404f22d89ab2a20ce0698271ff))
    - more progress on parity docs. ([`c5089ba`](https://github.comgit//DioxusLabs/dioxus/commit/c5089ba3c5a8daad4de4d6257604011cc87f6ac7))
    - dirty hack to enable send + sync on virtual dom ([`4d5c528`](https://github.comgit//DioxusLabs/dioxus/commit/4d5c528b07e61d6cb0ac8fc0c27ce2e0fdf7e7d2))
    - doesnt share on thread ([`fe67ff9`](https://github.comgit//DioxusLabs/dioxus/commit/fe67ff9fa4c9d5009670c922e192dccedb7cd09a))
    - recoil ([`ee67654`](https://github.comgit//DioxusLabs/dioxus/commit/ee67654f58aecca91c640fc49451d0b99bc05981))
    - Todomvc in progress ([`b843dbd`](https://github.comgit//DioxusLabs/dioxus/commit/b843dbd3679abf86a34347d87fd4ce5fe9e2aca5))
    - introduce children for walking down the tree ([`0d44f00`](https://github.comgit//DioxusLabs/dioxus/commit/0d44f009b0176cb9cf8203374cf534f5af7de63c))
    - context api wired up ([`24805a0`](https://github.comgit//DioxusLabs/dioxus/commit/24805a02f6a46539ff76ddcd569ecfc336ca2171))
    - about to consolidate context and scope ([`4c8130c`](https://github.comgit//DioxusLabs/dioxus/commit/4c8130c4e48a4f0eaef53c696154958d6e0ec6f2))
    - remove old code ([`3de54d0`](https://github.comgit//DioxusLabs/dioxus/commit/3de54d0b5202aca678d485a68ef8de006a63e21b))
    - abstraction lifetimes work out nicely ([`2284b35`](https://github.comgit//DioxusLabs/dioxus/commit/2284b357823336aafd8c622b466cd4695b78c690))
    - Clean up repo a bit ([`a99147c`](https://github.comgit//DioxusLabs/dioxus/commit/a99147c85b53b4ee336a94deee463d793cebf572))
    - some code health ([`c28697e`](https://github.comgit//DioxusLabs/dioxus/commit/c28697e1fe3136d1835f2b663715f34aab9f4b17))
    - major overhaul to diffing ([`9810fee`](https://github.comgit//DioxusLabs/dioxus/commit/9810feebf57f93114e3d7faf6de053ac192593a9))
    - Wip ([`c809095`](https://github.comgit//DioxusLabs/dioxus/commit/c809095124d06175e7f49853c34e48d7335573c5))
    - refactor a bit ([`2eeb8f2`](https://github.comgit//DioxusLabs/dioxus/commit/2eeb8f2386409b53836fab687097d89af7b883c3))
    - todos ([`8c541f6`](https://github.comgit//DioxusLabs/dioxus/commit/8c541f66d5f7ef2286f2cdf9b0496a9c404471f9))
    - todomvc ([`cfa0927`](https://github.comgit//DioxusLabs/dioxus/commit/cfa0927cdd40bc3dba22996018605dbad91d0391))
    - todomvc ([`ce33031`](https://github.comgit//DioxusLabs/dioxus/commit/ce33031519fbbbd207f1dffb75acf62bf59e3c9e))
    - more ergonomics, more examples ([`0bcff1f`](https://github.comgit//DioxusLabs/dioxus/commit/0bcff1f88e4b1a633b7a9b7c6c2e39b8bd3666c4))
    - use rsx! inline! ([`44aad27`](https://github.comgit//DioxusLabs/dioxus/commit/44aad2746c117ba9742c86a53327f4f9e96509e7))
    - building large apps, revamp macro ([`9f7f43b`](https://github.comgit//DioxusLabs/dioxus/commit/9f7f43b6614aaef2d7dded7058e81934f28f5dec))
    - begint to accept iterator types ([`742f150`](https://github.comgit//DioxusLabs/dioxus/commit/742f150eb3eba89913f5a0fabb229e72e2a0a5ee))
    - livehost bones ([`7856f2b`](https://github.comgit//DioxusLabs/dioxus/commit/7856f2b1537b52478a6fa1ca55d0a8c5793a91e5))
    - some stuff related to event listeners. POC for lifecyel ([`5b7887d`](https://github.comgit//DioxusLabs/dioxus/commit/5b7887d76c714d11e0cea5207197ffaac856a0a7))
    - diffing approach slightly broken ([`4e48e05`](https://github.comgit//DioxusLabs/dioxus/commit/4e48e0514e1caed9f60bcac4048114008ac76439))
    - remove old macro ([`9d0727e`](https://github.comgit//DioxusLabs/dioxus/commit/9d0727edabbf759dac8a3cffabd7103d08b728c1))
    - update examples ([`39bd185`](https://github.comgit//DioxusLabs/dioxus/commit/39bd1856f4d7c2364d6ca2c7812ab5d6e71252b9))
    - ensure mutabality is okay when not double-using the components ([`305ff91`](https://github.comgit//DioxusLabs/dioxus/commit/305ff919effa919ff7ea5db2a71cf21eca106588))
    - props now autoderives its own trait ([`b3c96a5`](https://github.comgit//DioxusLabs/dioxus/commit/b3c96a5996f434332813c737bb83ad564d91af5f))
    - somewhat working with rc and weak ([`d4f1cea`](https://github.comgit//DioxusLabs/dioxus/commit/d4f1ceaffbc0551ea3b179a101885275690cebec))
    - foregin eq from comparapable comp type. ([`ec801ea`](https://github.comgit//DioxusLabs/dioxus/commit/ec801eab167f9be5325ab877e74e2b65d501e129))
    - staticify? ([`5ad8188`](https://github.comgit//DioxusLabs/dioxus/commit/5ad81885e499bf02ac79e0098f7956d02ee5f2e5))
    - cargo fix to clean up things ([`78d093a`](https://github.comgit//DioxusLabs/dioxus/commit/78d093a9454386397a991bd01e603e4ad554521f))
    - implement vcomp fully ([`29751a4`](https://github.comgit//DioxusLabs/dioxus/commit/29751a4babfb46da25446ecf4077bfe9aa695655))
    - add some docs ([`5abda91`](https://github.comgit//DioxusLabs/dioxus/commit/5abda91892f32fd82a88d902aaefaccaabf07c7d))
    - update component so build passes ([`8fcd001`](https://github.comgit//DioxusLabs/dioxus/commit/8fcd001677146d83dc8e89402495c4e2a0ccc2f8))
    - wire up props macro ([`37f5a7a`](https://github.comgit//DioxusLabs/dioxus/commit/37f5a7ad33e272a9e210bf480304d54ff0df0d67))
    - still a bit stumped on DFS vs BFS ([`3740f81`](https://github.comgit//DioxusLabs/dioxus/commit/3740f81383b45e3300edda9c04f414852a828d12))
    - Feat:  it's awersome ([`8dc2619`](https://github.comgit//DioxusLabs/dioxus/commit/8dc26195e274874a7de6c806372f9f77a5c82c5d))
    - revert FC changes (like the old style). ([`7158bc3`](https://github.comgit//DioxusLabs/dioxus/commit/7158bc3575e180dbe8641549b040e74ae3baf80b))
    - dyn scope ([`89f2290`](https://github.comgit//DioxusLabs/dioxus/commit/89f22906926ccb5aa4cbe3fe1b6d834b06dd91e7))
    - yeet, synthetic somewhat wired up ([`d959806`](https://github.comgit//DioxusLabs/dioxus/commit/d9598066c2679d9d0b9ca0ce1d3f26110a238cd2))
    - synthetic events wired up (ish) ([`3087813`](https://github.comgit//DioxusLabs/dioxus/commit/30878135707ea86702a2f1f8c6f269c14576879f))
    - remove FC ([`92d9521`](https://github.comgit//DioxusLabs/dioxus/commit/92d9521a73aefb620b354ae5954617109dd06e7e))
    - notes on safety, and inline listeners ([`bdd6be3`](https://github.comgit//DioxusLabs/dioxus/commit/bdd6be309e8fa4c05a03c5cfc164f7564e7c3051))
    - cleanup edit module ([`c70652a`](https://github.comgit//DioxusLabs/dioxus/commit/c70652a3c91411e816012f79291926a716615d5c))
    - more cleanup ([`5a9155b`](https://github.comgit//DioxusLabs/dioxus/commit/5a9155b059acc1fb3c8b8accbeca3701ce4f0ab6))
    - add context to builder ([`cf16090`](https://github.comgit//DioxusLabs/dioxus/commit/cf16090838d127354e333dcbc0b06474835b87d6))
    - listeners now have scope information ([`fcd68e6`](https://github.comgit//DioxusLabs/dioxus/commit/fcd68e61d2400628469ba193b009e7bf1fd3acdf))
    - broken, but solved ([`cb74d70`](https://github.comgit//DioxusLabs/dioxus/commit/cb74d70f831b5510f1ee191d91eaff621ffa6256))
    - use_reducer ([`879e107`](https://github.comgit//DioxusLabs/dioxus/commit/879e107634f596d555f62f8853afcf5b3796b180))
    - wowza got it all working ([`4b8e9f4`](https://github.comgit//DioxusLabs/dioxus/commit/4b8e9f4a125b9d55439d919786f33d9d5df234e8))
    - parse custom rsx syntax ([`da00df6`](https://github.comgit//DioxusLabs/dioxus/commit/da00df66889f4fa2e39651491e08794e1fe78549))
    - update readme and examples ([`ffaf687`](https://github.comgit//DioxusLabs/dioxus/commit/ffaf6878963981860089c2362947bf77a84c9058))
    - view -> render ([`c8bb392`](https://github.comgit//DioxusLabs/dioxus/commit/c8bb392cadb22ddb41e53a776ab0677945579a9c))
    - bump core version ([`4997976`](https://github.comgit//DioxusLabs/dioxus/commit/499797626144738ec088ced0ca272c2e7a920c19))
    - remove html crate ([`9dcee01`](https://github.comgit//DioxusLabs/dioxus/commit/9dcee01b335901cf2c80b453b97180e0d2551dc2))
    - add core macro crate ([`c32a6ef`](https://github.comgit//DioxusLabs/dioxus/commit/c32a6ef7fe50d8901a0585c7e426ff0b16b1de6e))
    - bump version for web release ([`422d5ac`](https://github.comgit//DioxusLabs/dioxus/commit/422d5ac5af4c3d05627edd36a623663db158c3dd))
    - a few bugs, but the event system works! ([`3b30fa6`](https://github.comgit//DioxusLabs/dioxus/commit/3b30fa61b8f1927f22ffec373a3405ddfdefde39))
    - patch to diff to allow scopes ([`2041c88`](https://github.comgit//DioxusLabs/dioxus/commit/2041c88d07e8d68494bc77a10eaeb4707da078d9))
    - moving to CbIdx as serializable event system ([`e840f47`](https://github.comgit//DioxusLabs/dioxus/commit/e840f472faf25d8e1d65abeb610daed6a522771d))
    - custom format_args for inlining variables into html templates ([`e4b1f6e`](https://github.comgit//DioxusLabs/dioxus/commit/e4b1f6ea0d0db707cf757dabf8635e9fc91a3e0f))
    - begin WIP on html macro ([`a8b1225`](https://github.comgit//DioxusLabs/dioxus/commit/a8b1225c4853a61c7862e1a33eba64b8ed4e06d5))
    - move webview logic into library ([`32b45e5`](https://github.comgit//DioxusLabs/dioxus/commit/32b45e5ba168390e338a343eef6baba5cca9468b))
    - comments ([`18a7a1f`](https://github.comgit//DioxusLabs/dioxus/commit/18a7a1f9c40eb3b1ac263cbd21b1daa6da9c7093))
    - wire up rebuild ([`06ae4fc`](https://github.comgit//DioxusLabs/dioxus/commit/06ae4fc17833ff9695b0645f940b5ea32eeb4ef0))
    - clean up code ([`8345137`](https://github.comgit//DioxusLabs/dioxus/commit/83451372aa28ba4e137b553f07838a2db06c92da))
    - fix internal lifecycle ([`5204862`](https://github.comgit//DioxusLabs/dioxus/commit/5204862bc220e55afe8d2f8f4422cb530a6ea1cf))
    - add css example ([`edf09c1`](https://github.comgit//DioxusLabs/dioxus/commit/edf09c1892f2521f41bf84372629e84bed86e921))
    - borrowing ([`7a4594e`](https://github.comgit//DioxusLabs/dioxus/commit/7a4594e237acc9864ca9426f6d7002a8ab223d83))
    - finally solve the component lifetime problem <3 ([`bdc25b5`](https://github.comgit//DioxusLabs/dioxus/commit/bdc25b581b12ee536933032803785d3531b13693))
    - WIP ctx ([`7a6aabe`](https://github.comgit//DioxusLabs/dioxus/commit/7a6aabe4f39aa38bec7b548d030c11b2d9f481cb))
    - desktop app wired up ([`b3e6886`](https://github.comgit//DioxusLabs/dioxus/commit/b3e68863514a33291556ce278b7255fdf53e8b50))
    - remove our use of ouroborous. ([`bcbb93b`](https://github.comgit//DioxusLabs/dioxus/commit/bcbb93b69764dbff0d6130dc0f32bcf58ad7807f))
    - re-enable stack machine approach ([`e3ede7f`](https://github.comgit//DioxusLabs/dioxus/commit/e3ede7fcbf7acb5d8762bcddb5f1499104bb0ce7))
    - WIP on deserialize ([`f22ff83`](https://github.comgit//DioxusLabs/dioxus/commit/f22ff8319078d283e675ff8dc735da5bf8efe0df))
    - wire up a very basic dom updater ([`c4e8d8b`](https://github.comgit//DioxusLabs/dioxus/commit/c4e8d8bb31d54d70c8da2f4a5e3e3926ff7df92b))
    - major overhaul to diffing, using a "diffing machine" now ([`4dfdf91`](https://github.comgit//DioxusLabs/dioxus/commit/4dfdf9123608c69b86a56acbd6d8810b0cf1918c))
    - remove generic paramter on VDOM ([`4c291a0`](https://github.comgit//DioxusLabs/dioxus/commit/4c291a0efdbaadf5c2212248367c0a78046e2f83))
    - wire up some of the changelist for diff ([`d063a19`](https://github.comgit//DioxusLabs/dioxus/commit/d063a199391383288e807ac03d333091bac6ba60))
    - event loop ([`ea2aa4b`](https://github.comgit//DioxusLabs/dioxus/commit/ea2aa4b0c97b3292178284e05756d1415902a9e2))
    - major overhaul, wire up event system ([`8295ac4`](https://github.comgit//DioxusLabs/dioxus/commit/8295ac4b3dd2954e2aa23ef3a1f2e7c740a29961))
    - overall API updates ([`f47651b`](https://github.comgit//DioxusLabs/dioxus/commit/f47651b32afb385297ddae00616f4308083a36e6))
    - push new component API and context API ([`125f542`](https://github.comgit//DioxusLabs/dioxus/commit/125f5426a4719bdbad42c6c166e3a6002a2ce3f0))
    - update builder ([`8329268`](https://github.comgit//DioxusLabs/dioxus/commit/8329268d39238f030dd703ab4614253e58339e0b))
    - update solved problems with context API ([`c97a964`](https://github.comgit//DioxusLabs/dioxus/commit/c97a9647e3ccc477f5068d567a17d540c0bd199c))
    - Feat: - integrate subscription service into context. - Update documentation ([`204f0d9`](https://github.comgit//DioxusLabs/dioxus/commit/204f0d9f16c97fb847b7265296b43db36411b393))
    - fix docs names ([`ee23ea6`](https://github.comgit//DioxusLabs/dioxus/commit/ee23ea6c3a43cb6a3f5f36d8eab3a0df0daeb760))
    - implememt nodes better ([`edbb33b`](https://github.comgit//DioxusLabs/dioxus/commit/edbb33b2ee3692cac155b9e206cd7c2d6382eb9d))
    - move out scope into its own file ([`c9d95dd`](https://github.comgit//DioxusLabs/dioxus/commit/c9d95dd1dc35bde6f395dd95a951a0260ecfbb8a))
    - capture pointer type ([`e939373`](https://github.comgit//DioxusLabs/dioxus/commit/e939373616680ae74948fa2c36388332c52d0d1f))
    - more updates to hooks ([`2c2882c`](https://github.comgit//DioxusLabs/dioxus/commit/2c2882c9a24ed22aa6d17800ad3b1c308eb6579f))
    - found a fast solution to hook state ([`4d01436`](https://github.comgit//DioxusLabs/dioxus/commit/4d01436c3f45ef32ea17fa06c363f4c76dc8c227))
    - comment out examples and move lifetime in FC type ([`62d4ad5`](https://github.comgit//DioxusLabs/dioxus/commit/62d4ad58787185032100a2d25e79b70f6ec97a3c))
    - include the helper ([`07341d2`](https://github.comgit//DioxusLabs/dioxus/commit/07341d2c65dc61b90587e2e5daadf72ec82623a8))
    - updates to docs, extension ([`a2406b3`](https://github.comgit//DioxusLabs/dioxus/commit/a2406b33d6c11174f95bd003a59650b86f14159f))
    - webview example ([`7730fd4`](https://github.comgit//DioxusLabs/dioxus/commit/7730fd4a8c0183699684704777fb581129eddd8e))
    - Dioxus-webview ([`9c01736`](https://github.comgit//DioxusLabs/dioxus/commit/9c0173689539210d14847613f9a1694e6cb34506))
    - add router ([`6aeea9b`](https://github.comgit//DioxusLabs/dioxus/commit/6aeea9b79081a2a711d8eee7e6edd299caf379ef))
    - dioxus frontend crate ([`4d7ac5b`](https://github.comgit//DioxusLabs/dioxus/commit/4d7ac5bb5d3aa1897c0f6c1f322aca08c0c791f0))
    - Add versioning for package ([`33a805d`](https://github.comgit//DioxusLabs/dioxus/commit/33a805da4024a02f7fd2c68d97ce270c31154461))
    - clean up naming ([`2afbfea`](https://github.comgit//DioxusLabs/dioxus/commit/2afbfea324317003c601a331bd759414cce6b742))
    - prep for cargo ([`be8d267`](https://github.comgit//DioxusLabs/dioxus/commit/be8d2678b58ceb15c30ce75de630d0398c8951c3))
    - include diffing and patching in Dioxus ([`f3c650b`](https://github.comgit//DioxusLabs/dioxus/commit/f3c650b073b57073d30b65fb4f7fe15a6c5b2be6))
    - more docs, dissolve vnode crate into dioxus-core ([`9c616ea`](https://github.comgit//DioxusLabs/dioxus/commit/9c616ea5c092a756a437ccf175c8b04ada50b1b6))
    - more docs, example, mroe nodes ([`d13e04c`](https://github.comgit//DioxusLabs/dioxus/commit/d13e04c9ff4c2a98e295d762d631d47e0c762049))
    - some updates, pull in the vnode enums ([`152bced`](https://github.comgit//DioxusLabs/dioxus/commit/152bced9f38576f359c9270efd868179298528bc))
    - WIP ([`ce34d0d`](https://github.comgit//DioxusLabs/dioxus/commit/ce34d0dfcda82d847a6032f8f42749b39e0fba18))
    - docs, code frm percy ([`2b9c8d0`](https://github.comgit//DioxusLabs/dioxus/commit/2b9c8d09d926ff6b5ad8a7e7b7b0b6f93bb8eb36))
</details>

## v0.1.3 (2021-12-15)

### Documentation

 - <csr-id-4de16c4779648e591b3869b5df31271ae603c812/> update local examples and docs to support new syntaxes
 - <csr-id-e495b09bf1c99950b5ea8647b371f296dc64afbe/> fix table
 - <csr-id-9874f342e2bc1116410142c691545d75b0e0d6fe/> more docs
 - <csr-id-54d050cf897131ddc4f53651f1aeedaf03f23f57/> strong improvmenets, first major section done
 - <csr-id-9b5f82af7dce18b75a781b86b188bc713860bb99/> start on components
 - <csr-id-460783ad549818a85db634ed9c39ffce210b98ec/> lnks to projects
 - <csr-id-583fdfa5618e11d660985b97e570d4503be2ff49/> big updates to the reference
 - <csr-id-d9e6d0925b30690212d1d690dfba288f1a694a27/> examples
 - <csr-id-0826fdfee13d26b08cb8a2fa45cbd59cc4f20c25/> more docs
 - <csr-id-70cd46dbb2a689ae2d512e142b8aee9c80798430/> move around examples
 - <csr-id-69f5cc3802af136729bc73e5c3d209270d41b184/> move into a fromjs tutorial
 - <csr-id-7102fe5f984fe8692cf977ea3a43e2973eed4f45/> add some more sources in the core implementation
 - <csr-id-e4c06ce8e893779d2aad0883a1bb27d193bc5985/> update cargo tomls

### New Features

 - <csr-id-96b3d56e09a38843f7dc6c1047fe2a3923bd8473/> move back to combined cx/props
 - <csr-id-19df1bda109aba03c40ff631263bcb7035004ca0/> bubbling
 - <csr-id-8acdd2ea830b995b608d8bac2ef527db8d40e662/> it compiles once more
 - <csr-id-74c6211408ea0db5916a4e0973a3f9a7208faa44/> should be functional across the boar
 - <csr-id-55e6dd9701d3710ff17a25581b99b8f159e609b9/> wire tasks up
 - <csr-id-c10c1f418bf21e0e54b53e2babeb37c672c06901/> wire up linked nodes
 - <csr-id-9d8c5ca5ab5784b3f17d7ee20a451ee68fd703d6/> it properly bubbles
 - <csr-id-fd93ee89c19b085a04307ef30217170518defa8e/> upgrade syntax
 - <csr-id-11757ddf61e1decb1bd1c2bb30455d0bd01a3e95/> fake bubbling
 - <csr-id-f2234068ba7cd915a00a81e41660d7d6ee1177cc/> events bubble now
 - <csr-id-2cf90b6903411e42f01a801f89037686194ee068/> pull children out of component definition
 - <csr-id-84fd0c616252bf29cd665782258530032b54d13a/> cleanuup
 - <csr-id-79503f15c5db04fa04575c8735941a2e3a75030b/> full html support
 - <csr-id-6b2645fd80ab20153e7bb51887443e994fbab5cb/> improve FC handling to allow lifetimes of parent
 - <csr-id-fda2ebc2a22965845e015384f39f34ce7cb3e428/> improve safety
 - <csr-id-9726a065b0d4fb1ede5b53a2ddd58c855e51539f/> massage lifetimes
 - <csr-id-efd0e9b5648c809057f339083ba9d454f810d483/> support desktop more completely
 - <csr-id-a2b0c50a343005c63c7032bcefb8323b78350bb9/> add update functionality to useref
 - <csr-id-5502429626023d0788cca352e94ac6ea67c2cb11/> desktop functioning well
 - <csr-id-4a4c7afca7e1beadd4b213214074fdb420eb0923/> shared state mechanisms
 - <csr-id-fac42339c272b0e430ebf4f31b6061a0635d3e19/> mutations
 - <csr-id-687cda1b6d9595357d1dc8720ebe921f61098d8f/> re-enable suspense
 - <csr-id-4a72b3140bd244da602deada1eeecded65ff5848/> amazingly awesome error handling
 - <csr-id-d7940aa2ac2017316d62e0f2eac0701dc6ad1f09/> proper handling of events
 - <csr-id-d717c22d9c5da7b6f343fede11faaf953a3a29e0/> keyed diffing!!
 - <csr-id-fac2e56ed63fc708a89e14df15f1fc32f58391a9/> update root props
 - <csr-id-c321532a6cef40b2d2e4adc8c7a55931b6755b08/> some docs, cleaning
 - <csr-id-1749eba8eb6c8ffa38ab3ad52160b637fe021e86/> more and more sophisticated diff
 - <csr-id-d618092e9d150589e61516f7bbb169f2db49d3f2/> a new vnode type for anchors
 - <csr-id-00231adfa2e1d67a9d7ae2fa61c33e3a22d51978/> code quality improvements for core
 - <csr-id-a2c7d17b0595769f60bc1c2bbf7cbe32cec37486/> mvoe away from compound context
 - <csr-id-31702dbf878dd0207d101f7869ebefd2bb9f6860/> wire up resource pool
 - <csr-id-e5c88fe3a49649ecb308decd14c2557963978619/> make hooks free-functions
 - <csr-id-bfdcb20437c385116fd640e46b6361d09d8d5ff8/> transition to cx, props
 - <csr-id-f782e142118fb7acf1b88a0f3fbb03e4a5e3e91e/> omg what a dumb mistake
 - <csr-id-4837d8e741343d26f31b55e4478a374dc761e538/> suspense!
 - <csr-id-4a0068f09918adbc299150edcf777f342ced0dd3/> bless up, no more segfaults
 - <csr-id-de9f61bcf48c0d6e35e46c337b72a713c9f9f7d2/> more suspended nodes!
 - <csr-id-7dfe89c9581f45a445f17f9fe4bb94e61f67e971/> wire up event delegator for webview
 - <csr-id-0a907b35c647b67172080023462770b761364d51/> fix some lifetime issues
 - <csr-id-80e6c256980eb3e8c32e30f3dbb43c8b3b9a9cf4/> move over to push based mechanism
 - <csr-id-e7238762ae518c5688f9339d11832d17f99ad553/> architecture document and edit list
 - <csr-id-3a57b942624afb8aa6650aeee05466c3c9ce967e/> task system works
   but I broke the other things :(
 - <csr-id-f457b7113129479cad577237ef21cb735fffe483/> rebuild doesn't return errors
 - <csr-id-4091846934b4b3b2bc03d3ca8aaf7712aebd4e36/> add aria
 - <csr-id-c79d9ae674e235c8e9c2c069d24902122b9c7464/> buff up html allowed attributes
 - <csr-id-bbcb5a0234dbce48ffeb64903c3ec04562a87ad6/> enable components in ssr
 - <csr-id-0479252a5fba96e554dcf2422a8566e248fc6593/> keyed diffing
 - <csr-id-9abb0470b7869019d539a2fc21da3872348ae38b/> static node infrastructure and ssr changes
 - <csr-id-e6f56563bc84516e017b3db06f11fab2549b9a50/> tests and benchmarks
 - <csr-id-db6d0184aa5aa612bd7229a06f72b4c2ec5f6409/> dedicated mutations module
 - <csr-id-1cc1679a6ba33ff68d45ad1e964b73230224bc23/> refactor out the hooks implementation
 - <csr-id-2ce0752a9cd253fee1c8b205400aa191b09c9dcb/> fix compiling
 - <csr-id-99d94b69aba192be7e41ebe891aca4bb4f6cae88/> move webview to wry
 - <csr-id-e4cdb645aad800484b19ec35ba1f8bb9ccf71d12/> beaf up the use_state hook
 - <csr-id-22e659c2bd7797ca5a822180aca0cb5d950c5287/> namespaced attributes
   this commit adds namespaced attributes. This lets us support attribute groups, and thus, inline styles.
   
   This namespaced attribute stuff is only available for styles at the moment, though it theoretically could be enabled for any other attributes.
 - <csr-id-904b26f7111c3fc66400744ff6192e4b20bf6d74/> add edits back! and more webview support!
   This commit adds a new type - the DomEdit - for serializing the changes made by the diffing machine. The architecture of how DomEdits fit into the cooperative scheduling is still TBD but it will allow us to build change lists without applying them immediately. This is more performant  and allows us to only render parts of the page at a time.
   
   This commit also adds more infrastructure around webview. Dioxus can now run on the web, generate static pages, run in the desktop, and run on mobile, with a large part of thanks to webview.
 - <csr-id-b5e5ef171aa9f8986fb4ab04d793eb63f557c4ae/> two calculator examples
 - <csr-id-7665f2c6cf05cea64bb9131381d4ac11cbdeb932/> move to slotmap
 - <csr-id-f4fb5bb454536d9f108c7e276ce98a8924ab45e1/> integrate serialization and string borrowing
   This commit adds lifetimes to the diff and realdom methods so consumers may borrow the contents of the DOM for serialization or asynchronous modifications.
 - <csr-id-9d7ee79826a3b3fb952a70abcbb16dcd3363d2fb/> events work again!
 - <csr-id-73047fe95678d50fcfd62a4ace7c6b406c5304e1/> props memoization is more powerful
   This commit solves the memoization , properly memoizing properties that don't have any generic parameters. This is a rough heuristic to prevent non-static lifetimes from creeping into props and breaking our minual lifetime management.
   
   Props that have a generic parameter are opted-out of the `partialeq` requirement and props *without* lifetimes must implement partialeq. We're going to leave manual disabling of memoization for future work.
 - <csr-id-cfa0927cdd40bc3dba22996018605dbad91d0391/> todomvc
 - <csr-id-d4f1ceaffbc0551ea3b179a101885275690cebec/> somewhat working with rc and weak
 - <csr-id-89f22906926ccb5aa4cbe3fe1b6d834b06dd91e7/> dyn scope
 - <csr-id-8329268d39238f030dd703ab4614253e58339e0b/> update builder

### Bug Fixes

 - <csr-id-52c7154897111b570918127ffe3285bb1d5951a0/> really big bug around hooks
 - <csr-id-601078f9cf78a58d7502a377676ac94f3cf037bf/> desktop and mobile
 - <csr-id-77686ba329cba0679d0339589a00df77d3c4c1a1/> it compiles again
 - <csr-id-27d891934a70424b45e6278b7e2baaa2d1b78b35/> use annotation method from rust/58052 to fix closure lifetimes
 - <csr-id-ba9e1dbb8fa24048a6c9ccef8a8722688226a845/> messed up how lifetimes worked, need to render once per component
 - <csr-id-478255f40d4de1d2e3f3cc9b6d758b30ff394b39/> all the bugs!
 - <csr-id-a33f7701fcf5f917fea8719253650b5ad92554fd/> tags
 - <csr-id-868f6739d2b2c5f2ace0c5240cff8008901e818c/> keyword length

### Performance

 - <csr-id-8b3ac0b57ca073c1451e8d5df93882c9360ca52a/> remove global allocation for props
 - <csr-id-ea91fc984dc648150acbafd0abb79d9f42aca500/> refcell to cell for hookidx

