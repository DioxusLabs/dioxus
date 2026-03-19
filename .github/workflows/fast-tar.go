// Wrapper that delegates to Windows native bsdtar (C:\Windows\System32\tar.exe)
// instead of the slow MSYS2 tar from Git for Windows.
//
// Two transformations:
//   - Strips --force-local (GNU tar flag that bsdtar doesn't recognize)
//   - Handles --use-compress-program by decompressing with zstd ourselves,
//     then passing the plain .tar to bsdtar (bsdtar's CreateProcess-based
//     --use-compress-program doesn't work reliably on Windows)
package main

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
)

func main() {
	var args []string
	archive := ""
	decompress := ""
	skipNext := false

	for i, a := range os.Args[1:] {
		if skipNext {
			skipNext = false
			continue
		}
		switch a {
		case "--force-local":
			// bsdtar doesn't need this; drop it
		case "--use-compress-program":
			// Grab the program (e.g. "zstd -d") and handle it ourselves
			if i+1 < len(os.Args[1:]) {
				decompress = os.Args[1:][i+1]
			}
			skipNext = true
		case "-xf":
			// Next positional arg is the archive path
			args = append(args, a)
		default:
			// Track the archive filename (first bare arg after -xf)
			if len(args) > 0 && args[len(args)-1] == "-xf" && archive == "" {
				archive = a
				args = append(args, a)
			} else {
				args = append(args, a)
			}
		}
	}

	// If we have a compress program and a .tzst/.zst archive, decompress first
	if decompress != "" && archive != "" && (strings.HasSuffix(archive, ".tzst") || strings.HasSuffix(archive, ".zst")) {
		tarFile := strings.TrimSuffix(strings.TrimSuffix(archive, ".tzst"), ".zst") + ".tar"

		fmt.Fprintf(os.Stderr, "fast-tar: decompressing %s -> %s\n", archive, tarFile)
		zstd := exec.Command("zstd", "-d", archive, "-o", tarFile, "--force")
		zstd.Stdout = os.Stdout
		zstd.Stderr = os.Stderr
		if err := zstd.Run(); err != nil {
			fmt.Fprintf(os.Stderr, "fast-tar: zstd failed: %v\n", err)
			os.Exit(1)
		}
		defer os.Remove(tarFile)

		// Rewrite -xf to point at the decompressed .tar
		for i, a := range args {
			if a == archive {
				args[i] = tarFile
				break
			}
		}
	}

	fmt.Fprintf(os.Stderr, "fast-tar: C:\\Windows\\System32\\tar.exe %s\n", strings.Join(args, " "))
	cmd := exec.Command(`C:\Windows\System32\tar.exe`, args...)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	cmd.Stdin = os.Stdin
	if err := cmd.Run(); err != nil {
		if e, ok := err.(*exec.ExitError); ok {
			os.Exit(e.ExitCode())
		}
		os.Exit(1)
	}
}
