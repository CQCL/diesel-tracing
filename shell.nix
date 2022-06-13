{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "diesel-tracing-dev-env";
  buildInputs = with pkgs; [
    postgresql
    mysql
    sqlite
  ];

  LD_LIBRARY_PATH = "${pkgs.postgresql.lib}/lib";
}
