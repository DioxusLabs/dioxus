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

	function autoformat() {
		const editor = vscode.window.activeTextEditor;// Get the active text editor
		if (!editor) return;

		const unformatted = editor.document.getText(editor.selection);
		if (unformatted.length == 0) {
			vscode.window.showWarningMessage("Please select rsx invoking this command!");
			return;
		}

		const params = ["fmt", "--raw", unformatted];

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




	const handles =
		[
			vscode.commands.registerCommand('extension.htmlToDioxusRsx', () => translate(false)),
			vscode.commands.registerCommand('extension.htmlToDioxusComponent', () => translate(true)),
			vscode.commands.registerCommand('extension.formatRsx', () => autoformat())
		];

	context.subscriptions.push(...handles);
}
