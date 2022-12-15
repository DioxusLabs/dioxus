export function main() {
  let root = window.document.getElementById("main");
  if (root != null) {
    window.interpreter = new Interpreter(root);
    window.ipc.postMessage(serializeIpcMessage("initialize"));
  }
}
