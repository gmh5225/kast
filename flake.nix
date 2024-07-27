{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

    # Precisely filter files copied to the nix store
    nix-filter.url = "github:numtide/nix-filter";
  };

  outputs = { self, nixpkgs, nix-filter }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in
    {
      packages.${system} = {
        default = with pkgs; ocamlPackages.buildDunePackage {
          pname = "kast";
          version = "0.1.0";
          duneVersion = "3";
          src = nix-filter.lib {
            root = ./.;
            include = [
              ".ocamlformat"
              "dune-project"
              (nix-filter.lib.inDirectory "bin")
              (nix-filter.lib.inDirectory "lib")
              (nix-filter.lib.inDirectory "test")
            ];
          };
          buildInputs = [
            # Ocaml package dependencies needed to build go here.
            makeWrapper
          ];
          strictDeps = true;
          preBuild = ''
            dune build kast.opam
          '';
          postFixup = ''
            wrapProgram $out/bin/kast --set KAST_STD ${./std}
          '';
        };
      };
      devShells.${system} = {
        default = pkgs.mkShell {
          packages = with pkgs; with ocamlPackages; [
            # opam
            dune_3
            ocaml
            ocaml-lsp
            ocamlformat
            just
            rlwrap
            zola
            screen
          ];
          shellHook = ''
            echo Hello from Kast devshell
            mkdir -p .flock
            mkdir -p .logs
            echo "These services should now be running (you can check with screen -ls):"
            screen -L -Logfile .logs/zola -S zola -dm \
              flock --conflict-exit-code 0 --nonblock .flock/zola \
                bash -c "cd website && zola serve"
            echo "  zola: serving the website at http://127.0.0.1:1111"
            screen -L -Logfile .logs/dune -S dune -dm \
              flock --conflict-exit-code 0 --nonblock .flock/dune \
                bash -c \
                "
                  unset TMP;
                  unset TMPDIR;
                  unset TEMP;
                  unset TEMPDIR;
                  unset NIX_BUILD_TOP;
                  dune build -w;
                "
            echo "  dune: build --watch"
            # export OCAMLRUNPARAM=b
          '';
        };
      };
      formatter.${system} = pkgs.nixpkgs-fmt;
      checks.${system} = builtins.mapAttrs
        (name: _entry:
          let test = import ./tests/${name} { inherit pkgs; };
          in
          pkgs.stdenv.mkDerivation {
            name = "kast-check-${name}";
            nativeBuildInputs = [ self.packages.${system}.default ];
            dontUnpack = true;
            # doCheck = true;
            buildPhase = ''
              kast ${test.import or ""} ${test.source} < ${test.input or "/dev/null"} > $out
              diff $out ${test.expected_output}
            '';
          }
        )
        (builtins.readDir ./tests);
    };
}
