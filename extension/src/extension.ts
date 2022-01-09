'use strict';

import * as vscode from 'vscode';
import { spawn } from "child_process";

export function activate(context: vscode.ExtensionContext) {
	const htmlToPureRsx = vscode.commands.registerCommand('extension.htmlToRsx', function () {
		// Get the active text editor
		const editor = vscode.window.activeTextEditor;

		if (editor) {
			const document = editor.document;
			const selection = editor.selection;
			const word = document.getText(selection);

			const child_proc = spawn("dioxus", ["translate", "--source", word]);

			let result = '';
			child_proc.stdout?.on('data', data => result += data);

			child_proc.on('close', () => {
				editor.edit(editBuilder => {
					if (result != '') {
						editBuilder.replace(selection, result)
					}
				})
			});
		}
	});

	const htmlToComponent = vscode.commands.registerCommand('extension.htmlToComponent', function () {
		// Get the active text editor
		const editor = vscode.window.activeTextEditor;

		if (editor) {
			const document = editor.document;
			const selection = editor.selection;
			const word = document.getText(selection);

			const child_proc = spawn("dioxus", ["translate", "--component", "--source", word]);

			let result = '';
			child_proc.stdout?.on('data', data => result += data);

			child_proc.on('close', () => {
				editor.edit(editBuilder => {
					if (result != '') {
						editBuilder.replace(selection, result)
					}
				})
			});
		}
	});

	context.subscriptions.push(htmlToPureRsx);
	context.subscriptions.push(htmlToComponent);
}
