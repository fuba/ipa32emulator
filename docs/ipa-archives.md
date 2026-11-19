# IPA archive validation

IPA archives are opened with `zip` 9.2.3. Before creating the guest
filesystem, the emulator checks that the local ZIP headers agree with
the central directory and reports an error if they are inconsistent.
Entries without a usable modification timestamp use the ZIP default
of January 1, 1980.

Run `cargo test --locked -- --skip test_app` to check stored and
Deflate-compressed IPA entries and the dependency notices. The separate
`test_app` integration test requires the toolchain in
[tests/README.md](../tests/README.md).
