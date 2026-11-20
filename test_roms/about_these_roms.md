# About cartridge test roms

These roms are modificated versions of the rom `halt_bug.gb` available in the pack `gb-test-roms` you can find [here](https://github.com/retrio/gb-test-roms).

# More details
- `valid_rom.gb` is just `halt_bug.gb` with its name changed.
- `bad_checksum.gb` is a version of `halt_bug.gb` in which the byte `0x14D` has been changed.
- `short_rom.gb` is a version of `halt_bug.gb` in which almost all its bytes has been removed, keeping only the first `0x120` bytes.
- `minimal_rom.gb` is a version of `halt_bug.gb` in which only the first `0x150` bytes has been preserved (minimum rom size).