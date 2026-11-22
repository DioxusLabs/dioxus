const { execFileSync } = require("node:child_process");
const path = require("node:path");
module.exports = () => {
  const repo = path.resolve(__dirname, "../../..");
  execFileSync(
    "cargo",
    [
      "run",
      "--quiet",
      "-p",
      "dioxus-interpreter-js",
      "--example",
      "cleanup-fixture",
      "--features",
      "binary-protocol",
      "--",
      path.join(repo, "target/interpreter-cleanup-fixtures"),
    ],
    { cwd: repo, stdio: "inherit" },
  );
};
