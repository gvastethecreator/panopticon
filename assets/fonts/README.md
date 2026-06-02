# Bundled fonts

The TTF files in this directory belong to the **Miranda Sans** family and
are distributed under the **SIL Open Font License, Version 1.1**. The full
license text is in [`LICENSE-OFL.txt`](LICENSE-OFL.txt) and is also
available at <https://openfontlicense.org/>.

## Files

| File | Role |
| --- | --- |
| `MirandaSans-Regular.ttf`, `MirandaSans-Medium.ttf`, `MirandaSans-SemiBold.ttf`, `MirandaSans-Bold.ttf` | Static weights shipped at runtime. |
| `MirandaSans-{Regular,Medium,SemiBold,Bold}-Italic.ttf` | Italic static weights. |
| `MirandaSans-Variable.ttf`, `MirandaSans-Italic-Variable.ttf` | Variable-font originals retained for parity. They are not loaded at runtime; the Slint renderer currently consumes the static weights above. |

## Attribution

Panopticon does not modify the original font files; it only bundles and
references them. Please refer to the upstream project for the canonical
copyright line, the list of authors, and any Reserved Font Names. The
OFL requires that the license text accompanies the fonts and that
modifications are clearly marked — neither applies here.

If you redistribute Panopticon, the OFL text in this directory travels
with the fonts; no further action is required.
