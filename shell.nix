{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "diesel-tracing-dev-env";
  buildInputs = with pkgs; [
    pkg-config
    postgresql
    libmysqlclient
    sqlite
  ];

  LD_LIBRARY_PATH = "${pkgs.postgresql.lib}/lib:${pkgs.libmysqlclient.out}/lib/mariadb:${pkgs.sqlite.out}/lib";
}
