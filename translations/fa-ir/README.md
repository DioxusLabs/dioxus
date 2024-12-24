<p>
    <p align="center" >
      <img src="../../notes/header-light.svg#gh-light-mode-only" >
      <img src="../../notes/header-dark.svg#gh-dark-mode-only" >
      <a href="https://dioxuslabs.com">
          <img src="../../notes/dioxus_splash_8.avif">
      </a>
    </p>
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
    <a href="https://dioxuslabs.com/learn/0.6/guide"> Guide </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/translations/zh-cn/README.md"> 中文 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/translations/pt-br/README.md"> PT-BR </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/translations/ja-jp/README.md"> 日本語 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/translations/tr-tr"> Türkçe </a>
     <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/translations/fa-ir"> فارسی </a>
  </h3>
</div>
<br>
<br>

<div dir="rtl">
    <blockquote>
        یادداشت مترجم: اصطلاحات فنی را به زبان اصلی استفاده کرده و تا حد امکان توضیح داده‌ام تا هم برای افراد باتجربه و هم برای تازه‌کارها ساده‌تر شود. زبان تخصصی ما انگلیسی است، بنابراین اطلاعات داخل پرانتز عمدتاً برای دوستان تازه‌کار است. دوستان باتجربه معمولاً با اصطلاحات و معانی آن‌ها آشنا هستند. از آنجا که اصطلاحات فنی در زندگی روزمره نیز به همان شکل اصلی استفاده می‌شوند، ترجمه آن‌ها به فارسی کمی عجیب به نظر می‌رسد. البته ضروری هم نبود، اما تلاش کردم تا برای افرادی که آشنا نیستند، تصوری ایجاد کنم. وگرنه هیچ‌کس به جای frontend از کلمه‌ای فارسی یا به جای framework از جایگزینی دیگر استفاده نمی‌کند، من هم کاملاً این را می‌دانم. برای جلوگیری از ایجاد حس ناخوشایند، فقط در اولین برخورد با این مفاهیم آن‌ها را توضیح داده‌ام و در ادامه از شکل اصلی استفاده کرده‌ام. اگر خطایی وجود دارد، لطفاً ببخشید.
    </blockquote>
</div>

<div dir="rtl">
    وب، دسکتاپ و موبایل؛ همه را تنها با یک زیرساخت کدنویسی تولید کنید. نصب آسان، بارگذاری مجدد خودکار (hotreloading) که به‌طور خودکار با شناسایی تغییرات فعال می‌شود، و مدیریت وضعیت مبتنی بر سیگنال. با استفاده از توابع سرور، ویژگی‌های backend (بخش پشتی) را اضافه کنید و با CLI (رابط خط فرمان) بسته‌بندی کنید (با استفاده از دستور `dx bundle`).
</div>

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

<div dir="rtl">
  
## ⭐ ویژگی‌های منحصر به فرد:

- تولید برنامه‌های چندپلتفرمی (وب، دسکتاپ، موبایل، سرور و موارد دیگر) تنها با ۳ خط کد.
- [مدیریت وضعیت ارگونومیک](https://dioxuslabs.com/blog/release-050) با ترکیب بهترین ویژگی‌های React، Solid و Svelte.
- عملکرد فوق‌العاده با سریع‌ترین فریمورک wasm (WebAssembly) در Rust [sledgehammer](https://dioxuslabs.com/blog/templates-diffing).
- بسته‌بندی یکپارچه برای انتشار در وب، macOS، Linux و Windows (با دستور `dx bundle` به‌سادگی قابل انجام است).
- و موارد بیشتر! گشتی بزنید -> [تور Dioxus](https://dioxuslabs.com/learn/0.6/).

## بارگذاری مجدد (hot-reloading) آنی

تنها با یک دستور `dx serve` برنامه خود را اجرا کنید. تغییرات در مارک‌آپ و استایل‌ها (مانند HTML و CSS، یا در Dioxus با استفاده از rsx) را انجام دهید و نتیجه را به‌صورت آنی مشاهده کنید. اگرچه قابلیت‌های hot-reloading در Rust هنوز به‌طور کامل بهینه نیستند، اما با استفاده از [hot-lib-reloader](https://docs.rs/hot-lib-reloader/latest/hot_lib_reloader/) ممکن است.

<div align="center">
  <img src="../../notes/hotreload.gif" alt="بارگذاری مجدد آنی">
</div>

## بسته‌بندی برای وب و دسکتاپ

تنها با اجرای دستور `dx bundle` برنامه شما با حداکثر بهینه‌سازی کامپایل و بسته‌بندی می‌شود. از قابلیت‌های ایجاد فایل‌های [`.avif`، فشرده‌سازی `.wasm` و کوچک‌سازی](https://dioxuslabs.com/learn/0.6/guides/assets) و موارد بیشتر بهره‌مند شوید. برنامه‌های سبک وب (کمتر از [50kb](https://github.com/ealmloff/tiny-dioxus/)) و برنامه‌های دسکتاپ/موبایل با حجم کمتر از 15mb تولید کنید.

<div align="center">
  <img src="../../notes/bundle.gif" alt="بسته‌بندی برنامه">
</div>

## مستندات فوق‌العاده

ما زمان زیادی صرف کردیم تا مستنداتی واضح، خوانا و جامع ارائه دهیم. تمامی عناصر HTML و لیسنرها (listeners) با استفاده از MDN مستند شده‌اند ([برای جزئیات بیشتر کلیک کنید](https://developer.mozilla.org)) و برای به‌روز بودن، مستندات با خود Dioxus به‌صورت مداوم ادغام می‌شوند. برای راهنماها، مرجع‌ها، آموزش‌ها و موارد بیشتر به [وبسایت Dioxus](https://dioxuslabs.com/learn/0.6/) سر بزنید. نکته جالب: ما از وبسایت Dioxus به‌عنوان آزمایشگاهی برای تست ویژگی‌های جدید استفاده می‌کنیم -> [مشاهده کنید!](https://github.com/dioxusLabs/docsite).

<div align="center">
  <img src="../../notes/docs.avif" alt="مستندات Dioxus">
</div>

## تمرکز بر تجربه توسعه‌دهنده

Dioxus تمرکز زیادی بر تجربه توسعه‌دهنده دارد و ابزارهای متعددی برای بهبود آن ارائه می‌دهد. [افزونه VSCode](https://marketplace.visualstudio.com/items?itemName=DioxusLabs.dioxus) را توسعه دادیم که به شما امکان می‌دهد کدهای RSX را به‌صورت خودکار قالب‌بندی کنید، HTML را به RSX تبدیل کنید و موارد بیشتر. همچنین، CLI قدرتمندی توسعه دادیم که به شما کمک می‌کند برنامه‌های جدید بسازید، آن‌ها را سرو کنید و برای پلتفرم‌های مختلف بسته‌بندی کنید. در نقشه راه ما قابلیت‌های بیشتری مانند انتشار نیز پیش‌بینی شده است.

<div align="center">
  <img src="../../notes/autofmt.gif" alt="فرمت‌دهی خودکار">
</div>

## جامعه

Dioxus یک پروژه با جامعه‌ای فعال و پویا در [Discord](https://discord.gg/XgGxMSkvUM) و [GitHub](https://github.com/DioxusLabs/dioxus/issues) است. ما همیشه آماده پاسخگویی به سوالات شما هستیم، کمک می‌کنیم و برای شروع پروژه‌های شما آماده‌ایم. [SDK ما](https://github.com/DioxusLabs/dioxus-std) توسط جامعه توسعه داده شده و از طریق [سازمان GitHub](https://github.com/dioxus-community/) به بهترین برنامه‌های Dioxus دسترسی پیدا می‌کنید.

</div>

<div align="center">
  <img src="../../notes/dioxus-community.avif">
</div>

<div dir="rtl">

## تیم اصلی تمام‌وقت

Dioxus که به عنوان یک پروژه جانبی آغاز شد، اکنون به تیم کوچکی تبدیل شده که به‌صورت تمام‌وقت روی آن کار می‌کند. با حمایت FutureWei، Satellite.im و برنامه Github Accelerator، توانسته‌ایم به‌طور تمام‌وقت روی Dioxus تمرکز کنیم. برنامه بلندمدت ما ارائه ابزارهای شرکتی باکیفیت و پولی است تا Dioxus به پروژه‌ای خودکفا تبدیل شود. اگر شرکت شما علاقه‌مند به استفاده از Dioxus است و تمایل به همکاری با ما دارد، لطفاً با ما تماس بگیرید!

## پلتفرم‌های پشتیبانی‌شده

</div>

<div dir="rtl" align="center">
  <table style="width:100%">
    <tr>
      <td>
        <b>وب</b>
        <br />
        <em>پشتیبانی سطح ۱</em>
      </td>
      <td>
        <ul>
          <li>رندر مستقیم به DOM با استفاده از WebAssembly.</li>
          <li>پیش‌پردازش با رندر سمت سرور (SSR) و بازسازی سمت کاربر.</li>
          <li>برنامه‌های سبک (مانند "سلام دنیا") مشابه React، با حجمی حدود 50 کیلوبایت.</li>
          <li>سرور توسعه یکپارچه (`dx serve`) و بارگذاری مجدد آنی.</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
        <b>فول‌استک (Frontend و Backend)</b>
        <br />
        <em>پشتیبانی سطح ۱</em>
      </td>
      <td>
        <ul>
          <li>قابلیت‌های رندر سمت سرور، بازسازی و fallback با استفاده از Suspense.</li>
          <li>توابع سرور داخلی برای مدیریت بک‌اند.</li>
          <li>ادغام با Extractors، Middleware و Routing.</li>
          <li>سازگاری با موبایل و دسکتاپ.</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
        <b>دسکتاپ</b>
        <br />
        <em>پشتیبانی سطح ۱</em>
      </td>
      <td>
        <ul>
          <li>رندر با استفاده از Webview یا - به‌صورت آزمایشی - WGPU یا Freya (Skia).</li>
          <li>راه‌اندازی آسان تنها با `cargo run` یا `dx serve`.</li>
          <li>دسترسی مستقیم به سیستم بدون نیاز به IPC.</li>
          <li>پشتیبانی از macOS، Linux و Windows با فایل‌های اجرایی سبک (کمتر از 3 مگابایت).</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
        <b>رندر زنده (Liveview)</b>
        <br />
        <em>پشتیبانی سطح ۱</em>
      </td>
      <td>
        <ul>
          <li>رندر کامل برنامه یا تنها بخشی از آن در سمت سرور.</li>
          <li>ادغام با فریمورک‌های محبوب Rust مانند Axum و Warp.</li>
          <li>پشتیبانی از بیش از 10,000 برنامه و تأخیر بسیار کم.</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
        <b>موبایل</b>
        <br />
        <em>پشتیبانی سطح ۲</em>
      </td>
      <td>
        <ul>
          <li>رندر با استفاده از Webview یا - به‌صورت آزمایشی - WGPU یا Skia.</li>
          <li>پشتیبانی از iOS و Android.</li>
          <li>این ویژگی در حال حاضر بسیار آزمایشی است و به‌روزرسانی‌های بیشتری در طول سال 2024 ارائه خواهد شد.</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
        <b>ترمینال</b>
        <br />
        <em>پشتیبانی سطح ۲</em>
      </td>
      <td>
        <ul>
          <li>رندر برنامه‌ها مستقیماً در ترمینال، مشابه ink.js.</li>
          <li>بر اساس مدل‌های Flexbox و CSS مشابه مرورگرها.</li>
          <li>ابزارهای داخلی مانند ورودی متن، دکمه‌ها و سیستم فوکوس.</li>
        </ul>
      </td>
    </tr>
  </table>
</div>

<div dir="rtl">

## اجرای مثال‌ها

برای اجرای مثال‌های موجود در پوشه اصلی، کافی است دستور `cargo run --example <نام مثال>` را اجرا کنید. با این حال، پیشنهاد ما این است که dioxus-cli را نصب کنید و مثال‌ها را با استفاده از `dx serve` اجرا کنید، زیرا بسیاری از این مثال‌ها از پلتفرم وب نیز پشتیبانی می‌کنند. اگر می‌خواهید این مثال‌ها را برای وب اجرا کنید، باید تغییراتی در فایل Cargo.toml اعمال کنید یا ویژگی پیش‌فرض دسکتاپ را غیرفعال کنید.

## مقایسه Dioxus با دیگر فریمورک‌ها

ما تمامی فریمورک‌ها را دوست داریم و از پیشرفت آن‌ها در اکوسیستم Rust لذت می‌بریم. در این راستا، بسیاری از پروژه‌های ما توسط فریمورک‌های دیگر استفاده می‌شوند. به عنوان مثال، کتابخانه flex-box ما [Taffy](https://github.com/DioxusLabs/taffy) در پروژه‌هایی مانند [Bevy](https://bevyengine.org/)، [Zed](https://zed.dev/)، [Lapce](https://lapce.dev/)، [Iced](https://github.com/iced-rs/iced) و بسیاری دیگر استفاده شده است.

چند ویژگی که Dioxus را از دیگر فریمورک‌ها متمایز می‌کند:

- **مشابه React**: ما بر اساس مفاهیمی مانند کامپوننت‌ها، props (مشابه آرگومان‌های تابع) و hooks (دسترسی کامپوننت‌ها به ویژگی‌های فریمورک) ساخته‌ایم. این امر ما را بیشتر به Svelte نزدیک می‌کند تا SolidJS.
- **HTML و CSS**: برای تطبیق بهتر و دیگر ویژگی‌ها، ما HTML و CSS را به عنوان پایه انتخاب کرده‌ایم.
- **پلتفرم مستقل (Renderer-agnostic)**: [VirtualDOM سریع ما](https://dioxuslabs.com/blog/templates-diffing) به شما این امکان را می‌دهد که رندر را برای هر پلتفرمی تغییر دهید.
- **حمایت از همکاری (Collaborative)**: ما کتابخانه‌هایی مانند [Taffy](https://github.com/DioxusLabs/taffy)، [manganis](https://github.com/DioxusLabs/manganis)، [include_mdbook](https://github.com/DioxusLabs/include_mdbook)، و [blitz](http://github.com/dioxusLabs/blitz) تولید کرده‌ایم تا اکوسیستم را به صورت اشتراکی رشد دهیم.

### مقایسه Dioxus با Tauri

Tauri فریمورکی برای ایجاد برنامه‌های دسکتاپ (و به زودی موبایل) است که از فریمورک‌های وب مانند React، Vue و Svelte برای frontend استفاده می‌کند. هر زمان که نیاز به اجرای یک عملیات native داشته باشید، باید توابع Rust بنویسید و از frontend آن‌ها را فراخوانی کنید.

- **به صورت کامل Rust (Natively Rust)**: معماری Tauri شما را به استفاده از JavaScript یا WebAssembly محدود می‌کند. در Dioxus، کد Rust شما مستقیماً روی سیستم کاربر اجرا می‌شود. اعمالی مانند تولید thread، دسترسی به فایل‌ها و غیره بدون استفاده از IPC bridge امکان‌پذیر هستند.
- **اهداف مختلف**: Tauri نیازمند پشتیبانی از JavaScript و ابزارهای ساخت آن است. این محدودیت‌هایی بر عملیات شما اعمال می‌کند. Dioxus با تمرکز بر Rust، ویژگی‌هایی مانند توابع سرور، بسته‌بندی پیشرفته و رندر native را فراهم می‌کند.
- **کدهای مشترک (DNA مشترک)**: اگرچه Tauri و Dioxus پروژه‌های متفاوتی هستند، اما از کتابخانه‌های مشترک مانند Tao و Wry برای مدیریت پنجره‌ها و webview استفاده می‌کنند.

### مقایسه Dioxus با Leptos

Leptos یک فریمورک مشابه SolidJS و SolidStart برای توسعه برنامه‌های وب fullstack است. Dioxus و Leptos اهداف مشابهی برای وب دارند، اما تفاوت‌هایی کلیدی دارند:

- **مدل بازفعال‌سازی (Reactivity model)**: در حالی که Leptos از signals برای reactivity استفاده می‌کند، Dioxus از Virtual DOM و re-render بهره می‌برد. اگرچه signals از نظر تئوری کارآمدتر است، اما در عمل [الگوریتم‌های Virtual DOM Dioxus](https://dioxuslabs.com/blog/templates-diffing) تفاوت قابل توجهی ایجاد نمی‌کنند. [مقایسه‌ها نشان داده‌اند](https://krausest.github.io/js-framework-benchmark/2024/table_chrome_123.0.6312.59.html) که حتی Dioxus سریع‌تر است.
- **کنترل جریان (Control flow)**: در Leptos، signals باعث محدود شدن به primitives خاص می‌شود. این موضوع ممکن است اشکال‌زدایی خطاهای رابط کاربری (UI) را دشوار کند. در Dioxus، از iterators، حلقه‌های `for` معمولی و شرایط `if` Rust استفاده می‌کنید، و همچنان برنامه شما reactive باقی می‌ماند.

</div>

```rust
fn Counters() -> Element {
  let mut counters = use_signal(|| vec![0; initial_length]);

  rsx! {
    button { onclick: move |_| counters.push(counters.len()); "Add Counter" }
    ul {
      for idx in 0..counters.len() {
        li {
          button { onclick: move |_| counters[idx] += 1; "{counters[idx]}" }
          button { onclick: move |_| { counters.write().remove(idx); } "Remove" }
        }
      }
    }
  }
}
```

<div dir="rtl">
  <blockquote>
    <a href="https://book.leptos.dev/view/04_iteration.html#dynamic-rendering-with-the-for-component">
      اما برای Leptos این وضعیت نیازمند ردیابی کلیدها (key tracking)، استفاده از کامپوننت <code>&lt;For&gt;</code>، ایجاد سیگنال‌های جدید و در نهایت پاک‌سازی دستی حافظه است.
    </a>
  </blockquote>
</div>

```rust
fn Counters() -> Element {
    let initial_counters = (0..initial_length)
        .map(|id| (id, create_signal(id + 1)))
        .collect::<Vec<_>>();

    let (counters, set_counters) = create_signal(initial_counters);

    let add_counter = move |_| {
        let sig = create_signal(next_counter_id + 1);
        set_counters.update(move |counters| counters.push((next_counter_id, sig)));
        next_counter_id += 1;
    };

    view! {
        <div>
            <button on:click=add_counter>
                "Add Counter"
            </button>
            <ul>
                <For
                    each=counters
                    key=|counter| counter.0
                    children=move |(id, (count, set_count))| {
                        view! {
                            <li>
                                <button
                                    on:click=move |_| set_count.update(|n| *n += 1)
                                >
                                    {count}
                                </button>
                                <button
                                    on:click=move |_| {
                                        set_counters.update(|counters| {
                                            counters.retain(|(counter_id, (signal, _))| {

                                                if counter_id == &id {
                                                    signal.dispose();
                                                }
                                                counter_id != &id
                                            })
                                        });
                                    }
                                >
                                    "Remove"
                                </button>
                            </li>
                        }
                    }
                />
            </ul>
        </div>
    }
}
```

<div dir="rtl">

- **حالت `Copy` (روش ایجاد کپی)**: در نسخه‌های 0.1 تا 0.4 Dioxus، برای افزایش انعطاف‌پذیری مکانیزم borrow checker (سیستمی برای اطمینان از اعتبار متغیرها در استانداردهای Rust)، از ویژگی‌های lifetime (طول عمر در Rust، سیستمی برای کنترل موجودیت متغیرها در بازه مورد نیاز) استفاده می‌شد. این روش برای مدیریت event handlers (مدیریت‌کننده‌های رویداد) به‌خوبی عمل می‌کرد، اما در زمینه async (غیرهمزمان) پیچیدگی‌هایی ایجاد می‌کرد. با انتشار نسخه 0.5 Dioxus، مدل [`Copy`](https://crates.io/crates/generational-box) از Leptos وام گرفته شد.

- **اهداف متفاوت**: Dioxus یک رندرر برای وب، دسکتاپ، موبایل، LiveView و پلتفرم‌های متعدد دیگر ارائه می‌دهد. همچنین، کتابخانه‌های جامعه‌محور و یک SDK چندپلتفرمی را توسعه می‌دهیم. این دامنه وسیع منجر به کندتر شدن انتشار نسخه‌های جدید نسبت به Leptos می‌شود. Leptos بیشتر بر fullstack و وب تمرکز دارد و ویژگی‌هایی مانند HTML streaming (پخش HTML)، islands (جزایر: گروه‌بندی کامپوننت‌ها برای ارسال همراه با WebAssembly)، و کامپوننت‌هایی مانند `<Suspense />` و `<Form />` ارائه می‌دهد که مختص وب هستند. به‌طور کلی، برنامه‌های وب ساخته‌شده با Leptos، footprint (حجم کمتری) خواهند داشت.

- **زبان‌های خاص دامنه (DSLs)**: هر دو فریمورک برای وب مناسب هستند، اما Dioxus برای ایجاد UI از یک DSL مشابه Rust استفاده می‌کند، در حالی که Leptos ساختاری شبیه به HTML دارد. دلیل انتخاب این روش توسط Dioxus، استفاده از امکاناتی مانند code folding (باز و بسته کردن بخش‌هایی از کد) و syntax highlighting (برجسته‌سازی نحو کد) در محیط‌های توسعه یکپارچه (IDE) است. به‌طور کلی، DSL در Dioxus امکانات بیشتری ارائه می‌دهد. برای مثال، Dioxus به‌طور خودکار متون را ترکیب می‌کند، در حالی که Leptos نیازمند استفاده از closures (توابع ناشناس)، و ماکروهایی مانند `format!` یا `format_args!` است.

</div>

```rust
// dioxus
rsx! {
  div { class: "my-class", enabled: true, "سلام، {name}" }
}

// leptos
view! {
  <div class="my-class" enabled={true}>
    "سلام "
    {move || name()}
  </div>
}
```

<div dir="rtl">
### مقایسه Dioxus و Yew

Yew که برای توسعه برنامه‌های تک‌صفحه‌ای وب طراحی شده است، یکی از منابع الهام برای Dioxus بوده است. با این حال، محدودیت‌های معماری Yew باعث شد که Dioxus توسعه یابد.

- **برنامه‌های تک‌صفحه‌ای وب**: Yew به‌طور خاص برای توسعه برنامه‌های تک‌صفحه‌ای وب طراحی شده است و به همین دلیل محدود به وب است. اما Dioxus به دلیل پشتیبانی از برنامه‌های چندپلتفرمی fullstack، برای وب، دسکتاپ، موبایل و سرور مناسب است.

- **ابزارهای توسعه‌دهنده**: Dioxus ابزارهایی مانند فرمت خودکار، بارگذاری مجدد فوری و بسته‌بندی (bundler) ارائه می‌دهد.

- **پشتیبانی مداوم**: Dioxus با افزودن ویژگی‌های جدید و رفع اشکالات به‌صورت روزانه، به‌طور فعال در حال توسعه است.

### مقایسه Dioxus و egui

egui یک کتابخانه Rust برای ایجاد رابط‌های کاربری گرافیکی (GUI) چندپلتفرمی است که پروژه‌هایی مانند [Rerun.io](https://www.rerun.io) از آن استفاده می‌کنند.

- **Immediate vs Retained (به‌روزرسانی فوری در مقابل نگهداری)**: egui برای ایجاد رندر مجدد در هر فریم طراحی شده است. این معماری برای بازی‌ها و برنامه‌های تعاملی مناسب است، اما استایل و طرح‌بندی بین فریم‌ها حفظ نمی‌شود. در مقابل، Dioxus به‌عنوان یک فریمورک رابط کاربری retained عمل می‌کند، یعنی UI فقط یک‌بار تولید شده و در بین فریم‌ها به‌روزرسانی می‌شود. این روش باعث می‌شود Dioxus از فناوری‌های بومی وب مانند HTML و CSS استفاده کند و عملکرد بهتری در زمینه عمر باتری و کارایی داشته باشد.

- **سفارشی‌سازی**: در حالی که egui سیستم استایل و طرح‌بندی خاص خود را دارد، Dioxus به شما اجازه می‌دهد از HTML و CSS استفاده کنید. این ویژگی امکان استفاده از کتابخانه‌هایی مانند Tailwind و Material UI را فراهم می‌کند.

- **مدیریت وضعیت (State Management)**: egui از یک شیء وضعیت جهانی استفاده می‌کند، اما Dioxus از کامپوننت‌ها و props برای کپسوله‌سازی وضعیت‌ها و قابلیت استفاده مجدد از آنها بهره می‌برد.

### مقایسه Dioxus و Iced

Iced یک کتابخانه رابط کاربری گرافیکی چندپلتفرمی است که از Elm الهام گرفته شده و از WGPU برای رندر بومی و همچنین DOM nodes برای وب پشتیبانی می‌کند.

- **مدیریت وضعیت Elm**: Iced از مدل مدیریت وضعیت Elm استفاده می‌کند که شامل reducers (توابع) و پیام‌رسانی است. این معماری در مقایسه با Dioxus می‌تواند پرجزئیات‌تر و خسته‌کننده‌تر باشد.

- **رابط کاربری بومی**: Dioxus از webview به‌عنوان رندرر استفاده می‌کند و ویژگی‌هایی مانند کپی/چسباندن، دسترسی‌پذیری و متون بومی را ارائه می‌دهد. رندرر Iced به دلیل عدم پشتیبانی از این ویژگی‌ها، کمتر بومی به نظر می‌رسد.

- **WGPU**: رندرر WGPU در Dioxus هنوز به بلوغ کافی نرسیده و برای توسعه محصول مناسب نیست. اما رندرر WGPU در Iced بسیار پیشرفته‌تر است و برای توسعه محصول استفاده می‌شود.

### مقایسه Dioxus و Electron

Dioxus و Electron دو پروژه با اهداف مشابه اما معماری کاملاً متفاوت هستند. Electron به توسعه‌دهندگان اجازه می‌دهد با استفاده از فناوری‌های وب مانند HTML، CSS و JavaScript برنامه‌های دسکتاپ چندپلتفرمی ایجاد کنند.

- **سبک‌تر بودن**: Dioxus از webview بومی سیستم یا به‌صورت اختیاری از WGPU برای رندر رابط کاربری استفاده می‌کند. به‌طور مثال، در macOS، یک برنامه Electron معمولاً 100 مگابایت فضا اشغال می‌کند، در حالی که برنامه مشابه Dioxus فقط 15 مگابایت است.

- **بلوغ**: Electron با داشتن جامعه‌ای بزرگ و ابزارهای فراوان، یک پروژه بسیار بالغ است. در مقابل، Dioxus همچنان جوان است و نیاز به کار بیشتری دارد.

## مشارکت

- برای اطلاعات بیشتر درباره مشارکت، به [صفحه مشارکت در سایت ما](https://dioxuslabs.com/learn/0.6/contributing) مراجعه کنید.
- مشکلات خود را در [ردیاب مشکلات](https://github.com/dioxuslabs/dioxus/issues) گزارش دهید.
- به Discord [بپیوندید](https://discord.gg/XgGxMSkvUM) و سوالات خود را مطرح کنید!

<a href="https://github.com/dioxuslabs/dioxus/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=dioxuslabs/dioxus&max=30&columns=10" />
</a>

## مجوز

این پروژه تحت [مجوز MIT] ارائه شده است.

[مجوز MIT]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

مگر اینکه به‌طور خاص ذکر شود، تمام مشارکت‌های شما در Dioxus تحت این مجوز خواهد بود.

</div>
