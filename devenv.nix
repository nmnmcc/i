{ pkgs, lib, ... }:
{
  languages.rust = {
    enable = true;
    toolchainFile = ./rust-toolchain.toml;
  };

  languages.javascript = {
    enable = true;
    npm.enable = true;
  };

  packages = [
    pkgs.cmake # builds the vendored Binaryen in crates/ilang-binaryen
    pkgs.python3
    pkgs.wasmtime
    pkgs.wasm-tools
  ];

  # The browser tests use the browsers of Nix, which match the Playwright
  # version in package-lock.json, with their own libraries. Playwright's check
  # of the host looks for libraries of the system, which these do not use.
  env = {
    PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
    PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD = "1";
    PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "1";
  } // lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
    # WebKit's headless backend needs an EGL vendor; Mesa renders in software.
    __EGL_VENDOR_LIBRARY_FILENAMES = "${pkgs.mesa}/share/glvnd/egl_vendor.d/50_mesa.json";
  };

  enterTest = "./scripts/dev-verify";
}
