{
  description = "rhex - terminal hexagonal roguelike";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    flake-utils.url = "github:numtide/flake-utils";
    flakebox = {
      url = "github:rustshop/flakebox";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      flakebox,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        projectName = "rhex";
        flakeboxLib = flakebox.lib.mkLib pkgs {
          config = {
            github.ci.buildOutputs = [ ".#ci.${projectName}" ];
            ast-grep.enable = false;
            # The published flakebox does not yet expose vale.enable.
            just.importPaths = [ "justfile.custom.just" ];
            git.pre-commit.hooks.vale = ''
              just vale
            '';
            just.rules.watch.enable = false;
            linker.mold.enable = true;
            linker.wild.enable = false;
            toolchain.components = [
              "rustc"
              "cargo"
              "clippy"
              "rust-analyzer"
              "rust-src"
              "rustfmt"
            ];
          };
        };
        buildSrc = flakeboxLib.source.fromPaths {
          root = ./.;
          paths = [
            "Cargo.toml"
            "Cargo.lock"
            "src"
            "tests"
          ];
        };
        multiBuild = (flakeboxLib.craneMultiBuild { }) (
          craneLib':
          let
            craneLib = craneLib'.overrideArgs {
              pname = projectName;
              src = buildSrc;
              env.RUSTDOCFLAGS = "-D warnings";
            };
          in
          rec {
            workspaceDeps = craneLib.buildWorkspaceDepsOnly { };
            workspace = craneLib.buildWorkspace { cargoArtifacts = workspaceDeps; };
            tests = craneLib.cargoNextest {
              cargoArtifacts = workspace;
              cargoNextestExtraArgs = "--workspace";
              doInstallCargoArtifacts = false;
              nativeBuildInputs = [ pkgs.python3 ];
            };
            clippy = craneLib.cargoClippy {
              cargoArtifacts = workspaceDeps;
              cargoClippyExtraArgs = "--all-targets";
              doInstallCargoArtifacts = false;
            };
            cargoFmt = craneLib.cargoFmt { };
            ${projectName} = craneLib.buildPackage {
              cargoArtifacts = workspaceDeps;
            };
          }
        );
        treefmt =
          pkgs.runCommand "treefmt-check"
            {
              nativeBuildInputs = [
                pkgs.treefmt
                pkgs.nixfmt-rfc-style
                pkgs.rustfmt
                pkgs.taplo
              ];
              src = self;
            }
            ''
              cp -r $src work && chmod -R u+w work
              cd work
              treefmt --ci
              touch $out
            '';
      in
      {
        packages = {
          inherit treefmt;
          default = multiBuild.${projectName};
        };
        legacyPackages = multiBuild;
        devShells = flakeboxLib.mkShells {
          packages = [
            pkgs.taplo
            pkgs.python3
            pkgs.vale
          ];
        };
      }
    );
}
