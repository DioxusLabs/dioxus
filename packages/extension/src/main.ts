import * as vscode from 'vscode';
import init, { translate_rsx } from 'dioxus-ext';

export async function activate(context: vscode.ExtensionContext) {
	const wasmSourceCode = await vscode.workspace.fs.readFile(vscode.Uri.joinPath(context.extensionUri, "./pkg/dioxus_ext_bg.wasm"));
	const wasmPromise = await init(wasmSourceCode);

	context.subscriptions.push(
		vscode.commands.registerCommand('extension.htmlToDioxusRsx', () => translate(false)),
		vscode.commands.registerCommand('extension.htmlToDioxusComponent', () => translate(true)),
		// vscode.commands.registerCommand('extension.formatRsx', fmtSelection),
		// vscode.commands.registerCommand('extension.formatRsxDocument', formatRsxDocument),
		// vscode.workspace.onWillSaveTextDocument(fmtDocumentOnSave)
	);
}

function translate(component: boolean) {

	const editor = vscode.window.activeTextEditor;

	if (!editor) return;

	const html = editor.document.getText(editor.selection);
	if (html.length == 0) {
		vscode.window.showWarningMessage("Please select HTML fragment before invoking this command!");
		return;
	}

	const out = translate_rsx(html, component);

	if (out.length > 0) {
		editor.edit(editBuilder => editBuilder.replace(editor.selection, out));
	} else {
		vscode.window.showWarningMessage(`Errors occurred while translating, make sure this block of HTML is valid`);
	}
}



// function formatRsxDocument() {
// 	const editor = vscode.window.activeTextEditor;
// 	if (!editor) return;
// 	fmtDocument(editor.document);
// }

// function fmtSelection() {
// 	const editor = vscode.window.activeTextEditor;
// 	if (!editor) return;

// 	const unformatted = editor.document.getText(editor.selection);

// 	if (unformatted.length == 0) {
// 		vscode.window.showWarningMessage("Please select rsx invoking this command!");
// 		return;
// 	}

// 	const fileDir = editor.document.fileName.slice(0, editor.document.fileName.lastIndexOf('\\'));

// const child_proc = spawn(serverPath, ["fmt", "--raw", unformatted.toString()], {
// 	cwd: fileDir ? fileDir : undefined,
// });
// let result = '';

// child_proc.stdout?.on('data', data => result += data);

// child_proc.on('close', () => {
// 	if (result.length > 0) editor.edit(editBuilder => editBuilder.replace(editor.selection, result));
// });

// child_proc.on('error', (err) => {
// 	vscode.window.showWarningMessage(`Errors occurred while translating. Make sure you have the most recent Dioxus-CLI installed! \n${err}`);
// });
// }

// function fmtDocumentOnSave(e: vscode.TextDocumentWillSaveEvent) {
// 	// check the settings to make sure format on save is configured
// 	const dioxusConfig = vscode.workspace.getConfiguration('dioxus', e.document).get('formatOnSave');
// 	const globalConfig = vscode.workspace.getConfiguration('editor', e.document).get('formatOnSave');
// 	if (
// 		(dioxusConfig === 'enabled') ||
// 		(dioxusConfig !== 'disabled' && globalConfig)
// 	) {
// 		fmtDocument(e.document);
// 	}
// }

// function fmtDocument(document: vscode.TextDocument) {
// 	try {
// 		if (document.languageId !== "rust" || document.uri.scheme !== "file") {
// 			return;
// 		}

// 		const [editor,] = vscode.window.visibleTextEditors.filter(editor => editor.document.fileName === document.fileName);
// 		if (!editor) return; // Need an editor to apply text edits.

// 		const fileDir = document.fileName.slice(0, document.fileName.lastIndexOf('\\'));
// 		const child_proc = spawn(serverPath, ["fmt", "--file", document.fileName], {
// 			cwd: fileDir ? fileDir : undefined,
// 		});

// 		let result = '';
// 		child_proc.stdout?.on('data', data => result += data);

// 		/*type RsxEdit = {
// 			formatted: string,
// 			start: number,
// 			end: number
// 		}*/

// 		child_proc.on('close', () => {
// 			if (child_proc.exitCode !== 0) {
// 				vscode.window.showWarningMessage(`Errors occurred while formatting. Make sure you have the most recent Dioxus-CLI installed!\nDioxus-CLI exited with exit code ${child_proc.exitCode}\n\nData from Dioxus-CLI:\n${result}`);
// 				return;
// 			}

// 		});

// 		child_proc.on('error', (err) => {
// 			vscode.window.showWarningMessage(`Errors occurred while formatting. Make sure you have the most recent Dioxus-CLI installed! \n${err}`);
// 		});
// 	} catch (error) {
// 		vscode.window.showWarningMessage(`Errors occurred while formatting. Make sure you have the most recent Dioxus-CLI installed! \n${error}`);
// 	}
// }



// // I'm using the approach defined in rust-analyzer here
// //
// // We ship the server as part of the extension, but we need to handle external paths and such
// //
// // https://github.com/rust-lang/rust-analyzer/blob/fee5555cfabed4b8abbd40983fc4442df4007e49/editors/code/src/main.ts#L270
// async function bootstrap(context: vscode.ExtensionContext): Promise<string | undefined> {

// 	const ext = process.platform === "win32" ? ".exe" : "";
// 	const bundled = vscode.Uri.joinPath(context.extensionUri, "server", `dioxus${ext}`);
// 	const bundledExists = await vscode.workspace.fs.stat(bundled).then(
// 		() => true,
// 		() => false
// 	);

// 	// if bunddled doesn't exist, try using a locally-installed version
// 	if (!bundledExists) {
// 		return "dioxus";
// 	}

// 	return bundled.fsPath;
// }


// function onPasteHandler() {
// 	// check settings to see if we should convert HTML to Rsx
// 	if (vscode.workspace.getConfiguration('dioxus').get('convertOnPaste')) {
// 		convertHtmlToRsxOnPaste();
// 	}
// }

// function convertHtmlToRsxOnPaste() {
// 	const editor = vscode.window.activeTextEditor;
// 	if (!editor) return;

// 	// get the cursor location
// 	const cursor = editor.selection.active;

// 	// try to parse the HTML at the cursor location
// 	const html = editor.document.getText(new vscode.Range(cursor, cursor));
// }

/*if (result.length === 0) return;

// Used for error message:
const originalResult = result;
try {
	// Only parse the last non empty line, to skip log warning messages:
	const lines = result.replaceAll('\r\n', '\n').split('\n');
	const nonEmptyLines = lines.filter(line => line.trim().length !== 0);
	result = nonEmptyLines[nonEmptyLines.length - 1] ?? '';

	if (result.length === 0) return;

	const decoded: RsxEdit[] = JSON.parse(result);
	if (decoded.length === 0) return;

	// Preform edits at the end of the file
	// first (to not change previous text file
	// offsets):
	decoded.sort((a, b) => b.start - a.start);


	// Convert from utf8 offsets to utf16 offsets used by VS Code:

	const utf8Text = new TextEncoder().encode(text);
	const utf8ToUtf16Pos = (posUtf8: number) => {
		// Find the line of the position as well as the utf8 and
		// utf16 indexes for the start of that line:
		let startOfLineUtf8 = 0;
		let lineIndex = 0;
		const newLineUtf8 = '\n'.charCodeAt(0);
		// eslint-disable-next-line no-constant-condition
		while (true) {
			const nextLineAt = utf8Text.indexOf(newLineUtf8, startOfLineUtf8);
			if (nextLineAt < 0 || posUtf8 <= nextLineAt) break;
			startOfLineUtf8 = nextLineAt + 1;
			lineIndex++;
		}
		const lineUtf16 = document.lineAt(lineIndex);

		// Move forward from a synced position in the text until the
		// target pos is found:
		let currentUtf8 = startOfLineUtf8;
		let currentUtf16 = document.offsetAt(lineUtf16.range.start);

		const decodeBuffer = new Uint8Array(10);
		const utf8Encoder = new TextEncoder();
		while (currentUtf8 < posUtf8) {
			const { written } = utf8Encoder.encodeInto(text.charAt(currentUtf16), decodeBuffer);
			currentUtf8 += written;
			currentUtf16++;
		}
		return currentUtf16;
	};


	type FixedEdit = {
		range: vscode.Range,
		formatted: string,
	};

	const edits: FixedEdit[] = [];
	for (const edit of decoded) {
		// Convert from utf8 to utf16:
		const range = new vscode.Range(
			document.positionAt(utf8ToUtf16Pos(edit.start)),
			document.positionAt(utf8ToUtf16Pos(edit.end))
		);

		if (editor.document.getText(range) !== document.getText(range)) {
			// The text that was formatted has changed while we were working.
			vscode.window.showWarningMessage(`Dioxus formatting was ignored since the source file changed before the change could be applied.`);
			continue;
		}

		edits.push({
			range,
			formatted: edit.formatted,
		});
	}


	// Apply edits:
	editor.edit(editBuilder => {
		edits.forEach((edit) => editBuilder.replace(edit.range, edit.formatted));
	}, {
		undoStopAfter: false,
		undoStopBefore: false
	});

} catch (err) {
	vscode.window.showWarningMessage(`Errors occurred while formatting. Make sure you have the most recent Dioxus-CLI installed!\n${err}\n\nData from Dioxus-CLI:\n${originalResult}`);
}*/
// import { TextEncoder } from 'util';
