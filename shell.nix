let
  moz_overlay = import (builtins.fetchTarball
    "https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz");
  nixpkgs = import <nixpkgs> {
    overlays = [ moz_overlay ];
  };
  rustStableChannel =
    (nixpkgs.rustChannelOf { channel = "1.43.0"; }).rust.override {
      targets = [
        "x86_64-unknown-linux-gnu"
      ];
      extensions =
        [ "rust-src" "rls-preview" "clippy-preview" "rustfmt-preview" ];
    };
in with nixpkgs;
stdenv.mkDerivation {
  name = "chunked_hasher_shell";
  hardeningDisable = [ "all" ];
  buildInputs = [
    rustStableChannel
    pkgconfig
  ];
}
