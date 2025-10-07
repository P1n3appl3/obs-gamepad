{
  inputs = {
    utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, utils }: utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
      lib = pkgs.lib;
      native-deps = with pkgs; [ pkg-config rustPlatform.bindgenHook ];
      deps = with pkgs; [ libudev-zero libxkbcommon (lib.getLib obs-studio) ];
      lib-path = with pkgs; lib.makeLibraryPath [ xorg.libX11 xorg.libXcursor ];
    in
    {
      defaultPackage = pkgs.rustPlatform.buildRustPackage {
        pname = "obs-gamepad";
        version = "0.1.0";
        src = ./.;
        nativeBuildInputs = native-deps ++ [ pkgs.makeWrapper ];
        buildInputs = deps;
        cargoLock = {
          lockFile = ./Cargo.lock;
          outputHashes."serialport-4.7.3-alpha.0" = "sha256-JbPuIiCl21tHjwnZVMS50VWZ0Imzrre+OOV4KkTQ4hg=";
        };
        postInstall = ''
          mkdir -p $out/lib/obs-plugins
          mv $out/lib/libgamepad.so $out/lib/obs-plugins/obs-gamepad.so
          wrapProgram $out/bin/obs-gamepad --set LD_LIBRARY_PATH ${lib-path}
        '';
      };

      devShell = pkgs.mkShell {
        nativeBuildInputs = native-deps;
        buildInputs = deps ++ [ pkgs.yj ];
        LD_LIBRARY_PATH = lib-path;
      };

    }
  ) // {
    overlays.default = _: prev: {
      obs-studio-plugins = prev.obs-studio-plugins // {
        obs-gamepad = self.defaultPackage.${prev.system};
      };
    };
  };
}
