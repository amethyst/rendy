# Rendy iOS Triangle Example

This is a simple triangle example which aims to show a very basic application running on winit, displaying a triangle with rendy.

The Xcode project files are not included in this directory but the steps to configure a project to use the resulting Rust library are described below.

## Prerequisites

```
rustup target add aarch64-apple-ios
```

## Building

```
cd rendy/examples/ios_triangle
cargo build --release --target aarch64-apple-ios --features rendy/metal
```

The build currently fails with a CMake issue complaining about the C compiler not being able to compile and run a simple program.
For now you have to open up `CMakeCache.txt` in the target directory and replace `CMAKE_OSX_SYSROOT` with this value:

```
CMAKE_OSX_SYSROOT:PATH=/Applications/Xcode.app/Contents/Developer/Platforms/iPhoneOS.platform/Developer/SDKs/iPhoneOS13.0.sdk
```

Some links to reference about this oddity here:

[https://github.com/paritytech/rust-snappy/pull/10/files](https://github.com/paritytech/rust-snappy/pull/10/files)
[https://stackoverflow.com/questions/52879026/cmake-cross-compile-on-macos-adds-macos-sdk-to-isysroot-in-flags-make/52879604#52879604](https://stackoverflow.com/questions/52879026/cmake-cross-compile-on-macos-adds-macos-sdk-to-isysroot-in-flags-make/52879604#52879604)

* Open Xcode
* Make a new project
* Delete `AppDelegate.swift`
* Delete `SceneDelegate.swift` (if it exists, this is an iOS 13 SDK thing)
* Delete `ContentView.swift` (if it exists, this is an iOS 13 SDK thing)
* Delete "Application Scene Manifest" key from `Info.plist` (if it exists, this is an iOS 13 SDK thing)

#### Under Project Settings -> General
* Clear out "Main interface"

#### Under Project Settings -> Build phases -> Link Binary With Libraries
* Copy `$RUST_PROJECT/target/aarch64-apple-ios/release/libios_triangle.a` to `$IOS_PROJECT/ios-triangle/libs`
* Open `$IOS_PROJECT/libs` in Finder and drag `libios_triangle.a` to the "Link Binary With Libraries" section to add it
* Click the + and add `libc++.tbd`, `Metal.framework`, and `UIKit.framework` as well
* Set the "Objective-C Bridging Header" to `$IOS_PROJECT/ios-triangle/include/ios_triangle.h`

#### Under Project Settings -> Build Settings -> Search Paths
* Copy `$RUST_PROJECT/rendy/examples/ios_triangle/ios_triangle.h` to `$IOS_PROJECT/ios-triangle/include`
* Double click on "Header Search Paths" and add `$IOS_PROJECT/ios-triangle/include`
* Make sure "Library Search Paths" is also populated with `$IOS_PROJECT/ios-triangle/libs`
* Set "Enable Bitcode" to `NO`

#### In the project file explorer
* Add a new Swift file named `main.swift` with the contents of `main.swift` in this directory
* Don't create a bridging header
* Copy `$RUST_PROJECT/rendy/examples/ios_triangle/ios_triangle.h` to `$IOS_PROJECT/ios-triangle/include/ios_triangle.h`

Finally to build the Xcode project, set your build target to "Generic iOS Device" and select Product -> Build

You'll need to set a "development team" under "Signing & Capabilities" in order to successfully build and run.

When developing, you'll want some kind of script to copy `$RUST_PROJECT/target/aarch64-apple-ios/release/libios_triangle.a` to `$IOS_PROJECT/ios-triangle/libs` every time you build it.
