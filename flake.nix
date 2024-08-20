{
  inputs = {
    utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, utils }: utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
      lib = pkgs.lib;
    in
    {
      devShell = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          pkg-config
          rustPlatform.bindgenHook
        ];

        buildInputs = with pkgs; [
          libudev-zero
          libxkbcommon
          (lib.getLib obs-studio)
          yj
        ];

        LD_LIBRARY_PATH = with pkgs; lib.makeLibraryPath [
          xorg.libX11
          xorg.libXcursor
        ];
      };
    }
  );
}
