import * as vscode from 'vscode';
import { spawn } from "child_process";

export function activate(context: vscode.ExtensionContext) {


	function translate(component: boolean) {
		const editor = vscode.window.activeTextEditor;// Get the active text editor
		if (!editor) return;

		const html = editor.document.getText(editor.selection);
		if (html.length == 0) {
			vscode.window.showWarningMessage("Please select HTML fragment before invoking this command!");
			return;
		}

		let params = ["translate"];
		if (component) params.push("--component");
		params.push("--raw");
		params.push(html);

		const child_proc = spawn("dioxus", params);

		let result = '';
		child_proc.stdout?.on('data', data => result += data);

		child_proc.on('close', () => {
			if (result.length > 0) editor.edit(editBuilder => editBuilder.replace(editor.selection, result));
		});

		child_proc.on('error', (err) => {
			vscode.window.showWarningMessage(`Errors occurred while translating. Make sure you have the most recent Dioxus-CLI installed! \n${err}`);
		});
	}

	function autoformat_selection() {
		const editor = vscode.window.activeTextEditor;// Get the active text editor
		if (!editor) return;

		const unformatted = editor.document.getText(editor.selection);
		if (unformatted.length == 0) {
			vscode.window.showWarningMessage("Please select rsx invoking this command!");
			return;
		}

		let args = ["fmt", "--raw", unformatted.toString()];
		const child_proc = spawn("dioxus", args);
		let result = '';

		child_proc.stdout?.on('data', data => result += data);

		child_proc.on('close', () => {
			if (result.length > 0) editor.edit(editBuilder => editBuilder.replace(editor.selection, result));
		});

		child_proc.on('error', (err) => {
			vscode.window.showWarningMessage(`Errors occurred while translating. Make sure you have the most recent Dioxus-CLI installed! \n${err}`);
		});
	}


	function autoformat_document(document: vscode.TextDocument) {
		// check the settings to make sure format on save is configured
		// const formatOnSave: string | undefined = vscode.workspace.getConfiguration('rust').get('formatOnSave');

		if (document.languageId === "rust" && document.uri.scheme === "file") {
			const editor = vscode.window.activeTextEditor;// Get the active text editor
			if (editor) {
				const text = editor.document.getText();

				console.error(text);

				const child_proc = spawn("dioxus", ["fmt", "-f", text]);

				let result = '';

				child_proc.stdout?.on('data', data => result += data);

				type RsxEdit = {
					formatted: string,
					start: number,
					end: number
				}

				child_proc.on('close', () => {
					// if (result.length > 0) {
					// 	editor.edit(editBuilder => editBuilder.insert(new vscode.Position(0, 0), result));
					// } else {
					// 	console.error("No result");
					// }
					if (result.length > 0) {
						let decoded: RsxEdit[] = JSON.parse(result);

						console.log("Decoded edits: ", decoded);

						editor.edit(editBuilder => {

							for (let edit of decoded) {
								console.log("Handling Edit: ", edit);

								let start = document.positionAt(edit.start - 1);
								let end = document.positionAt(edit.end + 1);
								const range = new vscode.Range(start, end);

								editBuilder.replace(range, `{ ${edit.formatted}    }`);
								// editor.edit(editBuilder => editBuilder.replace(range, `{ ${edit.formatted}    }`)).then((could_be_applied) => {
								// });
								// editor.edit(editBuilder => editBuilder.replace(range, `{ ${edit.formatted}    }`)).then((could_be_applied) => {
								// 	console.log("Edit applied: ", could_be_applied);
								// });
							}
						})
					}
				});

				child_proc.on('error', (err) => {
					vscode.window.showWarningMessage(`Errors occurred while translating. Make sure you have the most recent Dioxus-CLI installed! \n${err}`);
				});
			}
		}
	}

	context.subscriptions.push(
		vscode.commands.registerCommand('extension.htmlToDioxusRsx', () => translate(false)),
		vscode.commands.registerCommand('extension.htmlToDioxusComponent', () => translate(true)),
		vscode.commands.registerCommand('extension.formatRsx', () => autoformat_selection()),
		vscode.commands.registerCommand('extension.formatRsxDocument', () => {
			const editor = vscode.window.activeTextEditor;// Get the active text editor
			if (!editor) return;
			autoformat_document(editor.document);
		}),
		vscode.workspace.onDidSaveTextDocument(autoformat_document)
	);
}
