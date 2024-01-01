{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  shellHook = ''export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath [
    pkgs.vulkan-loader
    # needed for now for smithay-client-toolkit dep
    pkgs.libxkbcommon
  ]}"'';

  buildInputs = with pkgs; [
    pkg-config
    wayland
  ];
}
