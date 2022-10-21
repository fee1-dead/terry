with import <nixpkgs> {};
stdenv.mkDerivation {
  name = "terry-env";
  buildInputs = [ pkg-config openssl gcc llvmPackages_14.llvm ];
  shellHook = ''
    export LLVM_SYS_140_PREFIX=${llvmPackages_14.llvm.dev}
  '';
}