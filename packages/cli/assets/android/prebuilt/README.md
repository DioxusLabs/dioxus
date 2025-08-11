This folder contains prebuilt versions of android crates to make cross-compiling easier.

We use the official prebuilds distributed by google.

You can find the full set of prebuilt libraries that google distributes here:

https://maven.google.com/web/index.html?q=com.android.ndk.thirdparty#com.android.ndk.thirdparty

The version included in `dx` is downloaded from here:

https://maven.google.com/web/index.html?q=com.android.ndk.thirdparty#com.android.ndk.thirdparty:openssl:1.1.1q-beta-1

The SHA of the `.aar` file from google and `dx` are different. The Rust openssl-sys crate expects libcrypto and libssl to be in the same folder, but the `.aar` that google distributes splits the two libraries into two different folders. I (jon) have simply merged the folders together and then re-packed the folder as a `.tar.gz`.
