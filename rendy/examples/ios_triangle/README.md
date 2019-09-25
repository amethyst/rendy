# Rendy iOS Triangle Example

This is a simple triangle example which aims to show a very basic application running on winit, displaying a triangle with rendy.

The Xcode project files are not included in this directory but the steps to configure a project to use the resulting Rust library are described below.

## Building

```
cd rendy/examples/ios_triangle
cargo build --release --target aarch64-apple-ios --features rendy/metal
```

* Open Xcode
* Make a new project
* Delete `AppDelegate.swift`
* Delete `SceneDelegate.swift` (if it exists, this is an iOS 13 SDK thing)
* Delete `ContentView.swift` (if it exists, this is an iOS 13 SDK thing)

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
	* Add a new Swift file
	* Don't create a bridging header
	* Copy `$RUST_PROJECT/rendy/examples/ios_triangle/ios_triangle.h` to `$IOS_PROJECT/ios-triangle/include/ios_triangle.h`

Finally to build the Xcode project, set your build target to "Generic iOS Device" and select Product -> Build

You'll need to set a "development team" under "Signing & Capabilities" in order to successfully build and run.

When developing, you'll want some kind of script to copy `$RUST_PROJECT/target/aarch64-apple-ios/release/libios_triangle.a` to `$IOS_PROJECT/ios-triangle/libs` every time you build it.
