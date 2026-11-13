<img src="res/branding/app-icon.svg" alt="IPA32 Emulator icon" width="96">

# IPA32 Emulator

IPA32 Emulator runs classic 32-bit iPhone apps on modern iPhones. This independent fork of [iDared 32bit](https://github.com/iDared32bit-emu/iDared32bit) adds offline compatibility for Dungeon Raid 1.3.4. It inherits the native iOS host, the ARM interpreter derived from [mGBA](https://github.com/mgba-emu/mgba), and the high-level emulator from [touchHLE](https://github.com/touchHLE/touchHLE).

The fork is based on iDared 32bit v1.5.2. See the [Dungeon Raid compatibility notes](docs/dungeon-raid.md) for the tested device, the offline game flow, and limitations. Compatibility varies by app.

## Build and run

1. Install Xcode, Rust (including the `aarch64-apple-ios` target), and CMake. Run `git submodule update --init --recursive`.
2. For local builds and tests, supply the two guest libraries described in [dylibs/README.md](dylibs/README.md).
3. Open `iphone/iDared32bit.xcodeproj` in Xcode, select the `iDared 32bit` scheme, and configure your signing team and a bundle identifier you can sign. If CMake requires it, set `CMAKE_POLICY_VERSION_MINIMUM=3.5` for the Rust build.
4. Build and run on your iPhone. Import a supported app or game you are authorized to use through the emulator’s file picker. Game files belong in `Games` and save data goes in `Saves`.

The macOS development build uses `cargo build --locked --release`. Other platforms are inherited from upstream and have not been verified for this fork.

## License and attribution

iDared 32bit © 2026 iDared 32bit project contributors.
touchHLE © 2023–2026 touchHLE project contributors.

Inherited source files and modifications to MPL-covered files remain under the [Mozilla Public License 2.0](LICENSE). The standalone ICU and guest Security framework implementations, the Dungeon Raid checkpoint feature’s Swift files, the fork’s original artwork, and its artwork generation script are covered by the [MIT license](src/licenses/MIT-IPA32-Emulator.txt). Third-party components retain their own licenses and notices, including those in [dylibs/](dylibs/) and [vendor/mgba_arm/](vendor/mgba_arm/).

### Name, logo and app icon

The iDared 32bit name, logo and app icon are © 2026 Alex Martin, all rights reserved. The MPL-2.0 does not grant rights to the upstream brand. IPA32 Emulator uses its own name and artwork. The new SVG originals are in [res/branding/](res/branding/). Run `dev-scripts/render-branding.sh` on macOS to regenerate the platform assets.

# Thanks

We stand on the shoulders of giants. Thank you to:

* Everyone who has contributed to the project or supported any of its contributors financially.
* The authors of and contributors to the many libraries used by this project: [mGBA](https://github.com/mgba-emu/mgba) (whose ARM interpreter core, modified, is vendored in [vendor/mgba_arm/](vendor/mgba_arm/)), [rust-macho](https://github.com/flier/rust-macho), [SDL](https://libsdl.org/), [rust-sdl2](https://github.com/Rust-SDL2/rust-sdl2), [stb\_image](https://github.com/nothings/stb), Imagination Technologies' [PVRTC decompressor](https://github.com/powervr-graphics/Native_SDK/blob/master/framework/PVRCore/texture/PVRTDecompress.cpp), [hound](https://github.com/ruuda/hound), [Symphonia](https://github.com/pdeljanov/Symphonia), [RustType](https://gitlab.redox-os.org/redox-os/rusttype), [the Liberation fonts](https://github.com/liberationfonts/liberation-fonts), [the Noto CJK fonts](https://github.com/googlefonts/noto-cjk), [rust-plist](https://github.com/ebarnard/rust-plist), [nibarchive](https://github.com/michaelwright235/nibarchive), [quick-xml](https://github.com/tafia/quick-xml), [gl-rs](https://github.com/brendanzab/gl-rs), [cargo-license](https://github.com/onur/cargo-license), [cc-rs](https://github.com/rust-lang/cc-rs), [cmake-rs](https://github.com/rust-lang/cmake-rs), [cargo-ndk](https://github.com/bbqsrc/cargo-ndk), [cargo-ndk-android-gradle](https://github.com/willir/cargo-ndk-android-gradle), [md-5 and sha1](https://github.com/RustCrypto/hashes), [encoding_rs](https://github.com/hsivonen/encoding_rs), [tiny-skia](https://github.com/linebender/tiny-skia), [corosensei](https://github.com/Amanieu/corosensei), [uuid](https://github.com/uuid-rs/uuid) and the Rust standard library.
* The [Rust project](https://www.rust-lang.org/) generally.
* The various people out there who've documented the iPhone OS platform, officially or otherwise. Much of this documentation is linked to within this codebase!
* The iOS hacking/jailbreaking community.
* The Free Software Foundation, for making libgcc and libstdc++ copyleft and therefore saving this project from ABI hell.
* Many friends who took an interest in the project and gave suggestions and encouragement.
* Developers of early iPhone OS apps. What treasures you created!
* Apple, and NeXT before them, for creating such fantastic platforms.
