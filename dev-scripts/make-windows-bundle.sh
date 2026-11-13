#!/bin/sh
set -e

# Bundles the IPA32 Emulator executable with the basic set of files needed for
# it to run (the same ones found in the macOS .app bundle or Android APK).
# This does not prepare a full release.

if [[ $# == 1 ]]; then
    PATH_TO_BINARY="$1"
    shift

    rm -rf ipa32emulator_windows_bundle
    mkdir ipa32emulator_windows_bundle
    cp $PATH_TO_BINARY ipa32emulator_windows_bundle/ipa32emulator.exe
    cp -r ../dylibs ipa32emulator_windows_bundle/
    cp -r ../touchHLE_fonts ipa32emulator_windows_bundle/
    cp -r ../touchHLE_default_options.txt ipa32emulator_windows_bundle/
    # Where games go. Windows has no fixed place to create it in.
    mkdir ipa32emulator_windows_bundle/Games
else
    echo "Incorrect usage."
    exit 1
fi
