# hl

> Simple highlight tool

## Examples
Highlighted portions are marked with `(` and `)`

```shell
$ expac "%n %m" -l'\n' linux firefox llvm dmenu file jq base nix | numfmt --to iec --format "%f" --field=2 --padding=1 | hl -f1:size
linux (126M)
firefox (221M)
llvm (95M)
dmenu (52K)
file (8.4M)
jq (691K)
base (0)
nix (11M)
```

NOTE: /proc/cpuinfo uses a tab before the `:` character
```shell
$ hl -f1:red < /proc/cpuinfo | head -n3
processor   : (0)
vendor_id   : (GenuineIntel)
cpu (family  :) 6

$ hl -s': ' -f0:red < /proc/cpuinfo | head -n3
processor   : (0)
vendor_id   : (GenuineIntel)
cpu family  : (6)
```

