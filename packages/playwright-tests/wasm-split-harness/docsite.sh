cargo build --bin wasm-split-cli
CLI=./target/debug/wasm-split-cli

rm -rf docsite/chunks

# Run the wasm-split-cli on the with_body.wasm file
${CLI} split docsite/input.wasm docsite/bindgen/main_bg.wasm docsite/chunks

# copy the contents of the wasm_bindgen folder to the docsite folder
mv docsite/chunks/main.wasm docsite/chunks/main_bg.wasm # rename the main wasm file
cp -r docsite/bindgen/snippets docsite/chunks/snippets
cp docsite/bindgen/main.js docsite/chunks/main.js

python3 -m http.server 8080 --directory docsite
