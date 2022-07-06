import * as vscode from 'vscode';
import { spawn } from "child_process";

let serverPath: string = "dioxus";

export async function activate(context: vscode.ExtensionContext) {
	let somePath = await bootstrap(context);

	if (somePath == undefined) {
		await vscode.window.showErrorMessage('Could not find bundled Dioxus-CLI. Please install it manually.');
		return;
	} else {
		serverPath = somePath;
	}

	context.subscriptions.push(
		vscode.commands.registerTextEditorCommand('editor.action.clipboardPasteAction', onPasteHandler),
		vscode.commands.registerCommand('extension.htmlToDioxusRsx', translateBlock),
		vscode.commands.registerCommand('extension.htmlToDioxusComponent', translateComponent),
		vscode.commands.registerCommand('extension.formatRsx', fmtSelection),
		vscode.commands.registerCommand('extension.formatRsxDocument', formatRsxDocument),
		vscode.workspace.onWillSaveTextDocument(fmtDocument)
	);
}


function translateComponent() {
	translate(true)
}

function translateBlock() {
	translate(false)
}

function translate(component: boolean) {
	const editor = vscode.window.activeTextEditor;

	if (!editor) return;

	const html = editor.document.getText(editor.selection);
	if (html.length == 0) {
		vscode.window.showWarningMessage("Please select HTML fragment before invoking this command!");
		return;
	}

	let params = ["translate"];
	if (component) params.push("--component");
	params.push("--raw", html);

	const child_proc = spawn(serverPath, params);

	let result = '';
	child_proc.stdout?.on('data', data => result += data);

	child_proc.on('close', () => {
		if (result.length > 0) editor.edit(editBuilder => editBuilder.replace(editor.selection, result));
	});

	child_proc.on('error', (err) => {
		vscode.window.showWarningMessage(`Errors occurred while translating. Make sure you have the most recent Dioxus-CLI installed! \n${err}`);
	});
}

function onPasteHandler() {
	// check settings to see if we should convert HTML to Rsx
	if (vscode.workspace.getConfiguration('dioxus').get('convertOnPaste')) {
		convertHtmlToRsxOnPaste();
	}
}

function convertHtmlToRsxOnPaste() {
	const editor = vscode.window.activeTextEditor;
	if (!editor) return;

	// get the cursor location
	const cursor = editor.selection.active;

	// try to parse the HTML at the cursor location
	const html = editor.document.getText(new vscode.Range(cursor, cursor));
}

function formatRsxDocument() {
	const editor = vscode.window.activeTextEditor;
	if (editor) {
		fmtDocument();
	}
}

function fmtSelection() {
	const editor = vscode.window.activeTextEditor;
	if (!editor) return;

	const unformatted = editor.document.getText(editor.selection);

	if (unformatted.length == 0) {
		vscode.window.showWarningMessage("Please select rsx invoking this command!");
		return;
	}

	const child_proc = spawn(serverPath, ["fmt", "--raw", unformatted.toString()]);
	let result = '';

	child_proc.stdout?.on('data', data => result += data);

	child_proc.on('close', () => {
		if (result.length > 0) editor.edit(editBuilder => editBuilder.replace(editor.selection, result));
	});

	child_proc.on('error', (err) => {
		vscode.window.showWarningMessage(`Errors occurred while translating. Make sure you have the most recent Dioxus-CLI installed! \n${err}`);
	});
}

function fmtDocument() {
	const editor = vscode.window.activeTextEditor;
	const document = editor?.document;
	if (!document) return;

	// check the settings to make sure format on save is configured
	if (document.languageId === "rust" && document.uri.scheme === "file") {
		const active_editor = vscode.window.activeTextEditor;

		if (active_editor?.document.fileName === document.fileName) {
			const text = document.getText();
			const child_proc = spawn("dioxus", ["fmt", "-f", text]);

			let result = '';
			child_proc.stdout?.on('data', data => result += data);

			type RsxEdit = {
				formatted: string,
				start: number,
				end: number
			}

			child_proc.on('close', () => {
				if (result.length > 0) {
					let decoded: RsxEdit[] = JSON.parse(result);

					if (decoded.length > 0) {
						active_editor.edit(editBuilder => {
							decoded.map((edit) => {
								editBuilder.replace(new vscode.Range(
									document.positionAt(edit.start),
									document.positionAt(edit.end)
								), edit.formatted);
							});
						}, {
							undoStopAfter: false,
							undoStopBefore: false
						})
					}
				}
			});

			child_proc.on('error', (err) => {
				vscode.window.showWarningMessage(`Errors occurred while translating. Make sure you have the most recent Dioxus-CLI installed! \n${err}`);
			});
		}
	}
}


// I'm using the approach defined in rust-analyzer here
//
// We ship the server as part of the extension, but we need to handle external paths and such
//
// https://github.com/rust-lang/rust-analyzer/blob/fee5555cfabed4b8abbd40983fc4442df4007e49/editors/code/src/main.ts#L270
async function bootstrap(context: vscode.ExtensionContext): Promise<string | undefined> {

	const ext = process.platform === "win32" ? ".exe" : "";
	const bundled = vscode.Uri.joinPath(context.extensionUri, "server", `dioxus${ext}`);
	const bundledExists = await vscode.workspace.fs.stat(bundled).then(
		() => true,
		() => false
	);

	// if bunddled doesn't exist, try using a locally-installed version
	if (!bundledExists) {
		return "dioxus";
	}

	return bundled.fsPath;
}
