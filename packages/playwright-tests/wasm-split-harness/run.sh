# This file is a simple shell script that runs the bundle split process manually without the CLI involved
# it's not necessarily meant to work on your machine (sorry!)
#
# To hack on harness you need the `wasm-tools` CLI installed
# `cargo binstall wasm-tools`
#
# This script is also sensitive to where it's run from, so you *need* to be in the harness folder (running as `./run.sh`)

TARGET_DIR=../../../target

# build the harness
cargo rustc --package wasm-split-harness --target wasm32-unknown-unknown --profile wasm-split-release -- -Clink-args=--emit-relocs

# for a much smaller compile, you can crank up the flags. However, dioxus relies heavily on location detail, so we can't disable that
#
# -Zlocation-detail=none - we could compile with location detail off but if breaks our signals...
#
# cargo +nightly rustc \
#   -Z build-std=std,panic_abort \
#   -Z build-std-features="optimize_for_size" \
#   -Z build-std-features=panic_immediate_abort \
#   --target wasm32-unknown-unknown \
#   --no-default-features \
#   --profile wasm-split-release \
#   -- -Clink-args=--emit-relocs

# Build the wasm-split-cli. We are going to call it directly since it's so noisy to build it multiple times
cargo build --package wasm-split-cli --bin wasm-split-cli
CLI=$TARGET_DIR/debug/wasm-split-cli

# clear the workdir and assemble the new structure
rm -rf data/harness
mkdir -p data/harness/split
mkdir -p data/harness/split_not

# copy the output wasm file to the harness dir
cp $TARGET_DIR/wasm32-unknown-unknown/wasm-split-release/wasm-split-harness.wasm data/harness/input.wasm

# Run wasm-bindgen on this module, without splitting it
wasm-bindgen data/harness/input.wasm --out-dir data/harness/split_not --target web --out-name main --no-demangle --no-typescript --keep-lld-exports --keep-debug

# Run the wasm-split-cli on the with_body.wasm file
${CLI} split data/harness/input.wasm data/harness/split_not/main_bg.wasm data/harness/chunks

# copy over the chunks
paths=$(ls data/harness/chunks/ | grep "\.wasm")
for path in $paths
do

    path_without_ext=${path%.*}
    wasm-opt -Oz data/harness/chunks/$path -o data/harness/split/$path --enable-reference-types --memory-packing --debuginfo

    # remove stuff like manganis, etc
    wasm-tools strip data/harness/split/$path -o data/harness/split/$path

    # if you don't want names (making it harder to debug the outputs) use `--all`
    # wasm-tools strip data/harness/split/$path -o data/harness/split/$path --all
done


# rename the main chunk
mv data/harness/split/main.wasm data/harness/split/main_bg.wasm
cp data/harness/split_not/main.js data/harness/split/main.js
cp -r data/harness/split_not/snippets data/harness/split/snippets
cp data/harness/chunks/__wasm_split.js data/harness/split/__wasm_split.js

wasm-opt -Oz data/harness/split_not/main_bg.wasm -o data/harness/split_not/main_bg_opt.wasm --enable-reference-types --memory-packing --debuginfo

# Run wasm-strip to strip out the debug symbols
wasm-tools strip data/harness/split_not/main_bg_opt.wasm -o data/harness/split_not/main_bg_opt.wasm

# if you don't want names (making it harder to debug the outputs) use `--all`
# wasm-tools strip data/harness/split/$path -o strip data/harness/split_not/main_bg_opt.wasm --all

echo "===========================================================================\n"
ls -l data/harness/split_not/main_bg_opt.wasm | awk '{ printf("%07d -> ", $5);print $9}'
echo ""
ls -l data/harness/split | grep "\.wasm" | awk '{ printf("%07d -> ", $5);print $9}'
echo "\n==========================================================================="

# hope you have python3 installed :)
python3 -m http.server 9876 --directory data
