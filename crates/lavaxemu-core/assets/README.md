# Font resources

`LVM.bin` is the existing reference VM's ASCII and GB2312 font resource.

`wqx-icons-16.bin` contains the 527 built-in 16×16 NC/TC device icons, in
F8A1–FCFE and FDA1–FDD9 order. It supplies the device glyphs beyond the GB2312
range, including the heart and snail used by 先赌为快.

Source: [`data/icon_16.dat` in arucil/wqxtools](https://github.com/arucil/wqxtools/blob/f3584c515c999093717143f1c179264c6a492818/data/icon_16.dat),
commit `f3584c515c999093717143f1c179264c6a492818`. The encoding layout is
documented in that revision's [`docs/fonts.txt`](https://github.com/arucil/wqxtools/blob/f3584c515c999093717143f1c179264c6a492818/docs/fonts.txt).
The binary is unchanged. Its MIT license and copyright notice are preserved in
`wqx-icons-LICENSE.txt`.
