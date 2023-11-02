import * as vscode from 'vscode';
import init, * as dioxus from 'dioxus-ext';

export async function activate(context: vscode.ExtensionContext) {
	// Load the wasm from the file system
	const wasmSourceCode = await vscode.workspace.fs.readFile(vscode.Uri.joinPath(context.extensionUri, "./pkg/dioxus_ext_bg.wasm"));

	// Wait for the initialization to finish
	// This is using the byte buffer directly which won't go through the "fetch" machinery
	//
	// For whatever reason, wasm-bindgen generates `fetch` when we don't want it to
	// VSCode doesn't have a `fetch` implementation, but we don't really care about polyfilling it
	await init(wasmSourceCode);

	// Todo:
	// I want a paste-handler that translates HTML to RSX whenever HTML is pasted into an Rsx block
	// Or, a little tooltip that pops up and asks if you want to translate the HTML to RSX
	context.subscriptions.push(
		vscode.commands.registerCommand('extension.htmlToDioxusRsx', () => translate(false)),
		vscode.commands.registerCommand('extension.htmlToDioxusComponent', () => translate(true)),
		vscode.commands.registerCommand('extension.formatRsx', fmtSelection),
		vscode.commands.registerCommand('extension.formatRsxDocument', formatRsxDocument),
		vscode.workspace.onWillSaveTextDocument(fmtDocumentOnSave)
	);
}

function translate(component: boolean) {
	// Load the activate editor
	const editor = vscode.window.activeTextEditor;
	if (!editor) return;

	// Get the selected text
	const html = editor.document.getText(editor.selection);
	if (html.length == 0) {
		vscode.window.showWarningMessage("Please select HTML fragment before invoking this command!");
		return;
	}

	// Translate the HTML to RSX
	const out = dioxus.translate_rsx(html, component);
	if (out.length > 0) {
		editor.edit(editBuilder => editBuilder.replace(editor.selection, out));
	} else {
		vscode.window.showWarningMessage(`Errors occurred while translating, make sure this block of HTML is valid`);
	}
}


function formatRsxDocument() {
	const editor = vscode.window.activeTextEditor;
	if (!editor) return;

	fmtDocument(editor.document);
}

function fmtSelection() {
	const editor = vscode.window.activeTextEditor;
	if (!editor) return;

	const unformatted = editor.document.getText(editor.selection);

	if (unformatted.length == 0) {
		vscode.window.showWarningMessage("Please select rsx invoking this command!");
		return;
	}

	const fileDir = editor.document.fileName.slice(0, editor.document.fileName.lastIndexOf('\\'));

}

function fmtDocumentOnSave(e: vscode.TextDocumentWillSaveEvent) {
	// check the settings to make sure format on save is configured
	const dioxusConfig = vscode.workspace.getConfiguration('dioxus', e.document).get('formatOnSave');
	const globalConfig = vscode.workspace.getConfiguration('editor', e.document).get('formatOnSave');
	if (
		(dioxusConfig === 'enabled') ||
		(dioxusConfig !== 'disabled' && globalConfig)
	) {
		fmtDocument(e.document);
	}
}

function fmtDocument(document: vscode.TextDocument) {
	try {
		if (document.languageId !== "rust" || document.uri.scheme !== "file") {
			return;
		}

		const [editor,] = vscode.window.visibleTextEditors.filter(editor => editor.document.fileName === document.fileName);
		if (!editor) return; // Need an editor to apply text edits.

		const contents = editor.document.getText();
		let tabSize: number;
		if (typeof editor.options.tabSize === 'number') {
			tabSize = editor.options.tabSize;
		} else {
			tabSize = 4;
		}
		const formatted = dioxus.format_file(contents, !editor.options.insertSpaces, tabSize);

		// Replace the entire text document
		// Yes, this is a bit heavy handed, but the dioxus side doesn't know the line/col scheme that vscode is using
		if (formatted.length() > 0) {
			editor.edit(editBuilder => {
				const range = new vscode.Range(0, 0, document.lineCount, 0);
				editBuilder.replace(range, formatted.formatted());
			});
		}
	} catch (error) {
		vscode.window.showWarningMessage(`Errors occurred while formatting. Make sure you have the most recent Dioxus-CLI installed! \n${error}`);
	}
}
