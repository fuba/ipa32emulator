# Dungeon Raid 1.3.4 on iPhone

This branch is based on iDared 32bit v1.5.2, commit `33c34bc138473050787b9518d6e07e23b18c0c4e`. The emulator runs the original ARMv7 game executable inside a locally signed ARM64 iOS app. The game includes its original OpenFeint code for local settings. The emulator defaults to offline operation.

Use the `iDared 32bit` scheme in `iphone/iDared32bit.xcodeproj`. The project was built with Rust 1.93.0, Xcode's iPhoneOS 27.2 SDK, and `CMAKE_POLICY_VERSION_MINIMUM=3.5`. Check out the pinned submodules with `git submodule update --init --recursive`. Set the signing team and bundle identifier for the destination device through Xcode or build settings.

Copy a locally prepared Dungeon Raid 1.3.4 IPA to the app's `Games` folder in Files. Open iDared 32bit and select the game. For device testing, `devicectl device process launch` can pass `--launch-ipa` and the filename after `--`. The original App Store package requires local preparation before import. Game files and Apple account data are outside this repository.

The new standalone `icu.rs` and `security.rs` implementations are available under MIT. Existing upstream files retain their license notices. The ICU wrapper uses `libicucore` from the Apple SDK on macOS and iOS. [ICU C regex reference](https://unicode-org.github.io/icu-docs/apidoc/released/icu4c/uregex_8h.html)

For a new game, choose **New Game**, enter a name or tap **Random**, choose a difficulty, and tap **Begin** at the bottom center. Tap **Begin** again on the introductory story. On the board, drag through three or more adjacent matching tiles. The app picker reopens from the circular exit button beneath the emulated screen.

The iPhone build preserves the entire 320×480 game viewport, including the top heading and bottom controls. The game opens with OpenFeint's offline choice as the fallback for unset preferences. Online OpenFeint and PlayHaven content is outside the offline game path.

Verified on a physical iPhone 18 Pro running iOS 27.2.1. A new game reached the board, a drag matched three coins and refilled their columns, and exiting to the title enabled **Continue**. The Rust library suite passed 79 tests on macOS.
