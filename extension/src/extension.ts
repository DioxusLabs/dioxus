import * as vscode from 'vscode';
import { spawn } from "child_process";

export function activate(context: vscode.ExtensionContext) {

	function registerCommand(cmd: string) {
		function convert(cmd: string) {
			const editor = vscode.window.activeTextEditor;// Get the active text editor
			if (editor) {
				const html = editor.document.getText(editor.selection);
				if (html.length > 0) {
					let params = ["translate"];
					if (cmd.includes("Component")) params.push("--component");
					params.push("--raw");
					params.push(html);
					const child_proc = spawn("dioxus", params);
					let result = '';
					child_proc.stdout?.on('data', data => result += data);
					child_proc.on('close', () => {
						if (result.length > 0) editor.edit(editBuilder => editBuilder.replace(editor.selection, result));
					});
				} else {
					vscode.window.showWarningMessage("Please select HTML fragment before invoking this command!");
				}
			}
		}


		const handle = vscode.commands.registerCommand(cmd, () => convert(cmd));
		context.subscriptions.push(handle);
	}

	registerCommand('extension.htmlToDioxusRsx');
	registerCommand('extension.htmlToDioxusComponent');
}
