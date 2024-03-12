<p align="center">
  <img src="../../notes/header.svg">
</p>

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/dioxus">
    <img src="https://img.shields.io/crates/v/dioxus.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/dioxus">
    <img src="https://img.shields.io/crates/d/dioxus.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs -->
  <a href="https://docs.rs/dioxus">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
  <!-- CI -->
  <a href="https://github.com/jkelleyrtp/dioxus/actions">
    <img src="https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg"
      alt="CI status" />
  </a>

  <!--Awesome -->
  <a href="https://dioxuslabs.com/awesome">
    <img src="https://cdn.rawgit.com/sindresorhus/awesome/d7305f38d29fed78fa85652e3a63e154dd8e8829/media/badge.svg" alt="Awesome Page" />
  </a>
  <!-- Discord -->
  <a href="https://discord.gg/XgGxMSkvUM">
    <img src="https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square" alt="Discord Link" />
  </a>
</div>

<div align="center">
  <h3>
    <a href="https://dioxuslabs.com"> Website </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/example-projects"> Examples </a>
    <span> | </span>
    <a href="https://dioxuslabs.com/learn/0.4/guide"> Guide </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/master/translations/zh-cn/README.md"> 中文 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/master/translations/pt-br/README.md"> PT-BR </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/master/translations/ja-jp/README.md"> 日本語 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/master/translations/tr-tr/README.md"> Türkçe </a>
  </h3>
</div>

<br/>

> [!WARNING]
> Dioxus 0.5 (şuan 'master' reposunda) uyumluluk kıran değişiklilere sahip ve Dioxus 0.4 ile uyumlu değil.


>Çevirmen Notu (Translator Note): Teknik terimleri orijinal haliyle kullanıp, olabildiğince açıklamaya çalıştım ki hem tecrübeli hem de yeni başlayan arkadaşlar için daha kolay olsun diye. Hatalar varsa affola.

Dioxus, Rust ile geliştirebileceğiniz, uyumlu, performanslı ve ergonomik bir frameworktür (geliştirme yapıları).

```rust
fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    }
}
```

Dioxus web, masaüstü, statik site, mobil uygulama, TUI(Termial User Interface(Terminal Arayüz Uygulamaları)), liveview(canlı görüntü(statik sayfanın gösterilip devamında sunucu ile desteklendiği)) gibi yapılarda uygulamalar geliştirmek için kullanılabilir. Dioxus renderer agnostic (platform bağımsızdır) ve her yerde platform görevi görebilir.

Eğer React biliyorsanız, Dioxus'u zaten biliyorsunuzdur.

## Nevi Şahsına Münhasır Özellikler:
- Masaüstü uygulamaları 10 satır koddan daha az yazılarak doğrudan çalıştırılabilir (Electron kullanmadan).
- İnanılmaz derecede ergonomic ve güçlü durum yönetimine sahiptir.
- Kod dahilinde kapsayıcı döküman - fareyi üzerine getirdiğinizde bütün HTML elementleri, listeners (takipçileri) ve events (olayları) için bilgi edinebilirsiniz. 
- Çok hızlı 🔥🔥 ve epey hafıza verimlidir.
- Hot-Reload (Derlemek zorunda olmadan değişikleri görme) sayesinde daha hızlı geliştirme imkanı sağlar.
- Coroutine(Eş zamanlı gibi hissettiren ama aslında sırayla yapılan işlemler) ve suspense(Yükleme bitene kadar halihazırda var olanı gösteren yapı) ile birinci sınıf asenkron desteği sunar.
- Daha fazlası için göz at: [Tam Sürüm Notu](https://dioxuslabs.com/blog/introducing-dioxus/).

## Desteklenen Platformlar
<div align="center">
  <table style="width:100%">
    <tr>
      <td><em>Web</em></td>
      <td>
        <ul>
          <li>WebAssembly ile doğrudan DOM (Sayfayı yenilemeden sayfa içeriği değiştirebilen yapı, aslında sayfanın durumu) manipülasyonu.</li>
          <li>SSR(Server Side Rendering (Sunucuda İşlemleri Gerçekleştirme)) ile pre-render (ön işleme) yapıp, client(sayfayı görüntüleyen, sınırlı işlem yapabilen kısım) tarafında sayfanın rehydrate(verilerin güncellenmesi) yapabilme.</li>
          <li>Basit bir "Hello World"(Merhaba Dünya) uygulaması 65kb yer kaplar. Bu değer React ile yarışabilecek seviyededir.</li>
          <li>Dahili dev server (geliştirme sunucusu(burada 'dx serve' den bahsediliyor)) ve hot-reload ile daha hızlı geliştirme imkanı.</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td><em>Masaüstü</em></td>
      <td>
        <ul>
          <li>Webview(web temelli render(işleme)) ya da deneysel olarak - WGPU ya da Skia - ile render yapabilirsiniz.</li>
          <li>Ayara ihtiyaç duymadan kurulum. Basitçe 'cargo-run' ile uygulamanızı çalıştırabilirsiniz. </li>
          <li>Electronsuz, doğrudan sisteme erişim için tam destek. </li>
          <li>Linux, macOS ve Windows desteklenir ve küçük çalıştırabilir dosyalar sunar. <3mb binary(derlenmiş çalıştırabilir) </li>
        </ul>
      </td>
    </tr>
    <tr>
      <td><em>Mobil</em></td>
      <td>
        <ul>
          <li>Webview ya da deneysel olarak - WGPU ya da Skia - ile render.</li>
          <li>Android ve iOS desteği. </li>
          <li><em>Gözle görülür şekilde</em> React Native'den daha performanslı. </li>
        </ul>
      </td>
    </tr>
    <tr>
      <td><em>Liveview</em></td>
      <td>
        <ul>
          <li>Tamamen server(sunucu) üzerinde uygulamalarınızı -ya da sadece bir parçasını- render edebilirsiniz. </li>
          <li>Axum ve Warp gibi popüler Rust frameworkleri ile entegrasyon.</li>
          <li>Çok iyi derecede düşük gecikme sağlar. 10.000+ uygulama eş zamanlı çalışabilir.</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td><em>Terminal</em></td>
      <td>
        <ul>
          <li>Uygulamaları doğrudan terminalde render edebilirsiniz, örnek:<a href="https://github.com/vadimdemedes/ink"> ink.js</a></li>
          <li>Flexbox ve CSS modelleri ile tarayıcıdaki yapılara benzer geliştirme şekilde geliştirme imkanı.</li>
          <li>Hazır olarak kullanılabilecek şekilde metin girdi kutuları, butonlar ve odak sistemi.</li>
        </ul>
      </td>
    </tr>
  </table>
</div>

## Neden Dioxus?
Uygulama geliştirmek için birsürü seçenek var, neden Dioxusu seçesin ?

Baktığımızda öncelikli olarak Dioxus geliştirici deneyimini önceliğinde tutar. Bu durum Dioxus üzerinde eşsiz birçok özellikte görülür:

- Kendi meta dilimiz (RSX) için oto-format ve [VSCode eklentisi](https://marketplace.visualstudio.com/items?itemName=DioxusLabs.dioxus).
- Hem masaüstü hem de web için hot-reload RSX interpretter(yorumlayıcı(derlemek yerine satır satır kodları çalıştıran)) kullanır.
- İyi dökümantasyona önem veriyoruz. Kod rehberi tamamlandı ve HTML elementleri de dökümente edildi.
- Basitletirmek için çaba gösteriyoruz.

Dioxus ayrıca eklentilere müsait bir platform.

- Basit bir optimize stack-machine() oluşturarak kendi rendererlarınızı kolayca geliştirebilirsiniz.
- Komponentleri ve hatta özel elementleri geliştirip paylaşabilirsiniz.

Yani... Dioxus güzel de, benim neden işime yaramıyor ?
- Gelişimi halen devam etmekte, APIlar değişim göstermekte, bir şeyler bozulabilir.(Bunu yapmamaya çalışsak bile)
- No-std(standart kütüphanesiz) bir ortamda çalıştırmanız gerekiyor.
- React tarzı hook modeli(diğer yapılara ve özelliklere erişmenizi sağlayan) ile arayüz geliştirmeyi sevmiyor olabilirsiniz.


## Katkı (Contributing)
- Websitemizi ziyaret edebilirsiniz. [Katkı kısmı](https://dioxuslabs.com/learn/0.4/contributing).
- Sorunlarınızı raporlayabilirsiniz. [Sorun takipçisi](https://github.com/dioxuslabs/dioxus/issues).
- [Katıl](https://discord.gg/XgGxMSkvUM) Discord'a ve sorularını sor!


<a href="https://github.com/dioxuslabs/dioxus/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=dioxuslabs/dioxus&max=30&columns=10" />
</a>

## Lisans
Bu proje [MIT license] ile lisanslanmıştır.

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Aksi açıkça belirtilmedikçe, yapılan ve Dioxus'a dahil edilen her türlü katkı (contribution) belirtilmesizin MIT lisansı ile lisanslanacaktır.