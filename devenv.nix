{ pkgs, lib, config, ... }:
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
    pkgs.ccache
    pkgs.python3
    pkgs.wasmtime
    pkgs.wasm-tools
  ];

  env = {
    # Binaryen's C++ is compiled from source by the build script of
    # ilang-binaryen, which Cargo runs again for clippy, for the tests, and for
    # release. ccache keeps the compiled objects, so the same sources are
    # compiled once. Paths in the checkout are hashed relative to it, so every
    # build directory of Cargo shares the objects, and the compiler is known by
    # its content, because every file in the Nix store has the same time.
    CMAKE_C_COMPILER_LAUNCHER = "ccache";
    CMAKE_CXX_COMPILER_LAUNCHER = "ccache";
    CCACHE_BASEDIR = config.devenv.root;
    CCACHE_COMPILERCHECK = "content";

    # The browser tests use the browsers of Nix, which match the Playwright
    # version in package-lock.json, with their own libraries. Playwright's
    # check of the host looks for libraries of the system, which these do not
    # use.
    PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
    PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD = "1";
    PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "1";
  } // lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
    # WebKit's headless backend needs an EGL vendor; Mesa renders in software.
    __EGL_VENDOR_LIBRARY_FILENAMES = "${pkgs.mesa}/share/glvnd/egl_vendor.d/50_mesa.json";
  };

  enterTest = "./scripts/dev-verify";
}
