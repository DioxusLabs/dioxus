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

	// Register format-on-save handler using waitUntil() for proper synchronization
	// This runs alongside (not instead of) other formatters like rust-analyzer,
	// allowing both rustfmt and Dioxus RSX formatting to work together
	context.subscriptions.push(
		vscode.workspace.onWillSaveTextDocument(formatOnSave)
	);

	// Todo:
	// I want a paste-handler that translates HTML to RSX whenever HTML is pasted into an Rsx block
	// Or, a little tooltip that pops up and asks if you want to translate the HTML to RSX
	context.subscriptions.push(
		vscode.commands.registerCommand('extension.htmlToDioxusRsx', () => translate(false)),
		vscode.commands.registerCommand('extension.htmlToDioxusComponent', () => translate(true)),
		vscode.commands.registerCommand('extension.formatRsx', fmtSelection),
		vscode.commands.registerCommand('extension.formatRsxDocument', formatRsxDocument)
	);

	context.subscriptions.push(vscode.window.registerUriHandler(new UriLaunchServer()));
}

// Format RSX on save using waitUntil() for proper synchronization with VSCode's save pipeline
function formatOnSave(e: vscode.TextDocumentWillSaveEvent) {
	if (e.document.languageId !== 'rust') {
		return;
	}

	// Check if Dioxus formatting is enabled
	const dioxusConfig = vscode.workspace.getConfiguration('dioxus', e.document).get('formatOnSave');
	if (dioxusConfig === 'disabled') {
		return;
	}

	// Use waitUntil() to properly synchronize with VSCode's save pipeline
	// This returns TextEdit[] which VSCode applies before completing the save
	e.waitUntil(formatDocument(e.document));
}

// Returns a promise of TextEdit[] for use with waitUntil()
function formatDocument(document: vscode.TextDocument): Thenable<vscode.TextEdit[]> {
	return new Promise((resolve) => {
		try {
			const contents = document.getText();

			// Get editor options for this document
			const editor = vscode.window.visibleTextEditors.find(
				e => e.document.uri.toString() === document.uri.toString()
			);

			const tabSize = (typeof editor?.options.tabSize === 'number') ? editor.options.tabSize : 4;
			const useTabs = editor ? !editor.options.insertSpaces : false;

			const formatted = dioxus.format_file(contents, useTabs, tabSize);

			if (formatted.length() > 0) {
				const fullRange = new vscode.Range(
					document.positionAt(0),
					document.positionAt(contents.length)
				);
				resolve([vscode.TextEdit.replace(fullRange, formatted.formatted())]);
			} else {
				resolve([]);
			}
		} catch (error) {
			vscode.window.showWarningMessage(`Dioxus formatting error: ${error}`);
			resolve([]);
		}
	});
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


async function formatRsxDocument() {
	const editor = vscode.window.activeTextEditor;
	if (!editor) return;

	// Apply RSX formatting directly using a WorkspaceEdit
	const edits = await formatDocument(editor.document);
	if (edits.length > 0) {
		const workspaceEdit = new vscode.WorkspaceEdit();
		for (const edit of edits) {
			workspaceEdit.replace(editor.document.uri, edit.range, edit.newText);
		}
		await vscode.workspace.applyEdit(workspaceEdit);
	}
}

function fmtSelection() {
	const editor = vscode.window.activeTextEditor;
	if (!editor) return;

	if (editor.document.languageId !== "rust") {
		return;
	}

	let end_line = editor.selection.end.line;

	// Select full lines of selection
	let selection_range = new vscode.Range(
		editor.selection.start.line,
		0,
		end_line,
		editor.document.lineAt(end_line).range.end.character
	);

	let unformatted = editor.document.getText(selection_range);

	if (unformatted.trim().length == 0) {
		vscode.window.showWarningMessage("Please select rsx invoking this command!");
		return;
	}

	// If number of closing braces is lower than opening braces, expand selection to end of initial block
	while ((unformatted.match(/{/g) || []).length > (unformatted.match(/}/g) || []).length && end_line < editor.document.lineCount - 1) {
		end_line += 1;

		selection_range = new vscode.Range(
			editor.selection.start.line,
			0,
			end_line,
			editor.document.lineAt(end_line).range.end.character
		);

		unformatted = editor.document.getText(selection_range);
	}

	let tabSize: number;
	if (typeof editor.options.tabSize === 'number') {
		tabSize = editor.options.tabSize;
	} else {
		tabSize = 4;
	}

	let end_above = Math.max(editor.selection.start.line - 1, 0);

	let lines_above = editor.document.getText(
		new vscode.Range(
			0,
			0,
			end_above,
			editor.document.lineAt(end_above).range.end.character
		)
	);

	// Calculate indent for current selection
	let base_indentation = (lines_above.match(/{/g) || []).length - (lines_above.match(/}/g) || []).length - 1;

	try {
		let formatted = dioxus.format_selection(unformatted, !editor.options.insertSpaces, tabSize, base_indentation);
		for (let i = 0; i <= base_indentation; i++) {
			formatted = (editor.options.insertSpaces ? " ".repeat(tabSize) : "\t") + formatted;
		}
		if (formatted.length > 0) {
			editor.edit(editBuilder => {
				editBuilder.replace(selection_range, formatted);
			});
		}
	} catch (error) {
		vscode.window.showErrorMessage(`Errors occurred while formatting. Make sure you have the most recent Dioxus-CLI installed and you have selected valid rsx with your cursor! \n${error}`);
	}

}

class UriLaunchServer implements vscode.UriHandler {
	handleUri(uri: vscode.Uri): vscode.ProviderResult<void> {
		if (uri.path === '/debugger') {
			let query = decodeURIComponent(uri.query);
			let params = new URLSearchParams(query);
			let route = params.get('uri');
			vscode.window.showInformationMessage(`Opening Chrome debugger: ${route}`);
			vscode.commands.executeCommand('extension.js-debug.debugLink', route);
		}
	}
}
