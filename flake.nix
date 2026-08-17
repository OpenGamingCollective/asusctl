{
  description = "Asusctl for Asus on Linux";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    fenix.url = "github:nix-community/fenix/monthly";
  };
  outputs =
    {
      self,
      nixpkgs,
      fenix,
    }:
    let
      supportedSystems = [ "x86_64-linux" ];
      forAllSystems = fn: nixpkgs.lib.genAttrs supportedSystems (system: fn system);
      pkgs = (system: nixpkgs.legacyPackages.${system});
      fenixpkgs = system: fenix.packages.${system};
      toolchainFor =
        system:
        let
          tc = (fenixpkgs system).toolchainOf {
            channel = "1.85.0";
            sha256 = "sha256-AJ6LX/Q/Er9kS15bn9iflkUwcgYqRQxiOIL2ToVAXaU=";
          };
        in
        (fenixpkgs system).combine [
          tc.cargo
          tc.rustc
          tc.rustfmt
          tc.clippy
          tc.rust-src
          tc.rust-analyzer
        ];
    in
    {
      devShells = forAllSystems (system: {
        default = (pkgs system).mkShell {
          packages = [
            (toolchainFor system)
            (pkgs system).gettext
            (pkgs system).pkg-config
            (pkgs system).udev
            (pkgs system).clang
            (pkgs system).SDL2
            (pkgs system).cargo-cranky
          ];
          LD_LIBRARY_PATH = (pkgs system).lib.makeLibraryPath [
            (pkgs system).wayland
            (pkgs system).libxkbcommon
            (pkgs system).vulkan-loader
            (pkgs system).libGL
            (pkgs system).udev
            (pkgs system).vulkan-headers
            (pkgs system).libxcb
            (pkgs system).egl-wayland
            (pkgs system).egl-x11
            (pkgs system).libglvnd
          ];
          LIBCLANG_PATH = "${(pkgs system).llvmPackages.libclang.lib}/lib";
          RUST_SRC_PATH = "${toolchainFor system}/lib/rustlib/src/rust/library";
        };
      });
    };
}
