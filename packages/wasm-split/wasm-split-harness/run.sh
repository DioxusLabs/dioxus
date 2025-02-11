# GENERATE_NAME_SECTION=true

# -Zlocation-detail=none - we could compile with location detail off but if breaks our signals...
cargo +nightly rustc \
  -Z build-std=std,panic_abort \
  -Z build-std-features="optimize_for_size" \
  -Z build-std-features=panic_immediate_abort \
  --target wasm32-unknown-unknown \
  --no-default-features \
  --profile wasm-split-release \
  -- -Clink-args=--emit-relocs

TARGET_DIR=../../../target

# build the harness
# cargo rustc --package wasm-split-harness --target wasm32-unknown-unknown --profile wasm-split-release -- -Clink-args=--emit-relocs

# Build the wasm-split-cli. We are going to call it directly since it's so noisy to build it multiple times
cargo build --package wasm-split-cli --bin wasm-split-cli
CLI=$TARGET_DIR/debug/wasm-split-cli

# clear the workdir
rm -rf data/harness/input.wasm
rm -rf data/harness/chunks
rm -rf data/harness/split
rm -rf data/harness/split_not
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

    # ~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/split/$path -R "names"
    ~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/split/$path -R "linking"
    ~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/split/$path -R "producers"
    ~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/split/$path -R "target_features"
    ~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/split/$path -R "reloc.CODE"
    ~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/split/$path -R "reloc.DATA"
    ~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/split/$path -R "__wasm_bindgen_unstable"
done


# rename the main chunk
mv data/harness/split/main.wasm data/harness/split/main_bg.wasm
cp data/harness/split_not/main.js data/harness/split/main.js
cp -r data/harness/split_not/snippets data/harness/split/snippets
cp data/harness/chunks/__wasm_split.js data/harness/split/__wasm_split.js

wasm-opt -Oz data/harness/split_not/main_bg.wasm -o data/harness/split_not/main_bg_opt.wasm --enable-reference-types --memory-packing --debuginfo

# Run wasm-strip to strip out the debug symbols
# ~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/split_not/main_bg_opt.wasm -R "names"
~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/split_not/main_bg_opt.wasm -R "linking"
~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/split_not/main_bg_opt.wasm -R "producers"
~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/split_not/main_bg_opt.wasm -R "target_features"
~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/split_not/main_bg_opt.wasm -R "reloc.CODE"
~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/split_not/main_bg_opt.wasm -R "reloc.DATA"

# echo "===========================================================================\n"
# ls -l data/harness/split_not/main_bg_opt.wasm | awk '{ printf("%07d -> ", $5);print $9}'
# echo ""
# ls -l data/harness/split | grep "\.wasm" | awk '{ printf("%07d -> ", $5);print $9}'
# echo "\n==========================================================================="

python3 -m http.server 8080 --directory data



# # # optimize the main chunk
# # wasm-opt -Oz data/harness/chunks/main.wasm -o data/harness/chunks/main.wasm --enable-reference-types --memory-packing

# # # Run wasm-strip to strip out the debug symbols
# # ~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/chunks/main_opt.wasm -R "linking"
# # ~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/chunks/main_opt.wasm -R "names"
# # ~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/chunks/main_opt.wasm -R "producers"
# # ~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/chunks/main_opt.wasm -R "target_features"
# # ~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/chunks/main_opt.wasm -R "reloc.CODE"
# # ~/Downloads/wabt-1.0.36/bin/wasm-strip data/harness/chunks/main_opt.wasm -R "reloc.DATA"
