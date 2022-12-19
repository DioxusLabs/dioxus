export function main(rootname = "main") {
  let root = window.document.getElementById(rootname);
  if (root != null) {
    window.interpreter = new Interpreter(root);
    window.ipc.postMessage(serializeIpcMessage("initialize"));
  }
}
