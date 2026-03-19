// Wrapper that delegates to Windows native bsdtar (C:\Windows\System32\tar.exe)
// instead of the slow MSYS2 tar from Git for Windows.
//
// Handles two GNU tar flags that bsdtar doesn't support well on Windows:
//   - --force-local: stripped (bsdtar doesn't need it)
//   - --use-compress-program "zstd -d": we pipe zstd stdout into bsdtar stdin
//     instead of letting bsdtar spawn the program (which hangs on Windows)
package main

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
)

func main() {
	var tarArgs []string
	archive := ""
	hasCompress := false
	skipNext := false
	nextIsArchive := false

	for i, a := range os.Args[1:] {
		if skipNext {
			skipNext = false
			continue
		}
		switch {
		case a == "--force-local":
			// drop it
		case a == "--use-compress-program":
			hasCompress = true
			skipNext = true // skip "zstd -d"
		case a == "-xf":
			nextIsArchive = true
			// don't append yet — we may rewrite to "-xf" "-"
		case nextIsArchive:
			archive = a
			nextIsArchive = false
			// don't append — handled below
		default:
			tarArgs = append(tarArgs, a)
			_ = i
		}
	}

	if hasCompress && archive != "" {
		// Pipe: zstd -d -c archive.tzst | bsdtar -xf - [other flags]
		tarArgs = append([]string{"-xf", "-"}, tarArgs...)
		fmt.Fprintf(os.Stderr, "fast-tar: zstd -d -c %s | tar.exe %s\n", archive, strings.Join(tarArgs, " "))

		zstd := exec.Command("zstd", "-d", "-c", archive)
		zstd.Stderr = os.Stderr

		tar := exec.Command(`C:\Windows\System32\tar.exe`, tarArgs...)
		tar.Stdout = os.Stdout
		tar.Stderr = os.Stderr

		pipe, err := zstd.StdoutPipe()
		if err != nil {
			fmt.Fprintf(os.Stderr, "fast-tar: pipe failed: %v\n", err)
			os.Exit(1)
		}
		tar.Stdin = pipe

		if err := tar.Start(); err != nil {
			fmt.Fprintf(os.Stderr, "fast-tar: tar start failed: %v\n", err)
			os.Exit(1)
		}
		if err := zstd.Run(); err != nil {
			fmt.Fprintf(os.Stderr, "fast-tar: zstd failed: %v\n", err)
			os.Exit(1)
		}
		if err := tar.Wait(); err != nil {
			if e, ok := err.(*exec.ExitError); ok {
				os.Exit(e.ExitCode())
			}
			os.Exit(1)
		}
	} else {
		// No compression — just forward to bsdtar directly
		if archive != "" {
			tarArgs = append([]string{"-xf", archive}, tarArgs...)
		}
		fmt.Fprintf(os.Stderr, "fast-tar: tar.exe %s\n", strings.Join(tarArgs, " "))
		cmd := exec.Command(`C:\Windows\System32\tar.exe`, tarArgs...)
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
}
