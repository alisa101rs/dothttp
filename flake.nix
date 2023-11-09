{
    description = "dothttp - cli tool for text-based http requests";
    inputs = {
         nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
         fenix = {
           url = "github:nix-community/fenix";
           inputs.nixpkgs.follows = "nixpkgs";
         };
         flake-utils.url = "github:numtide/flake-utils";
   };

    outputs = {
        self,
        nixpkgs,
        fenix,
        flake-utils,
    }: flake-utils.lib.eachDefaultSystem (system: {
         packages.default =
           let
             toolchain = fenix.packages.${system}.minimal.toolchain;
             pkgs = nixpkgs.legacyPackages.${system};
           in

           (pkgs.makeRustPlatform {
             cargo = toolchain;
             rustc = toolchain;
           }).buildRustPackage {
             pname = "dothttp";
             version = "0.6.0";

             src = ./.;
             nativeBuildInputs = [];
             cargoLock.lockFile = ./Cargo.lock;
           };
       });
}
