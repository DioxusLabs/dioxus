import * as vscode from 'vscode';
import * as dioxus from '../pkg';

export async function activate(context: vscode.ExtensionContext) {
  const channel = vscode.window.createOutputChannel('Dioxus');

  const path = vscode.Uri.joinPath(context.extensionUri, './dist/main.wasm');
  const content = await vscode.workspace.fs.readFile(path);

  dioxus.initSync(content);
  dioxus.initTracing(channel);

  context.subscriptions.push(
    channel,
    // This runs alongside (not instead of) other formatters like rust-analyzer, allowing both
    // `rustfmt` and Dioxus RSX formatting to work together.
    vscode.workspace.onWillSaveTextDocument(formatOnSave),
    vscode.commands.registerCommand('extension.htmlToDioxusRsx', () => {
      translate(false);
    }),
    vscode.commands.registerCommand('extension.htmlToDioxusComponent', () => {
      translate(true);
    }),
    vscode.commands.registerCommand('extension.formatRsx', fmtSelection),
    vscode.commands.registerCommand('extension.formatRsxDocument', formatRsxDocument),
    vscode.window.registerUriHandler(new UriLaunchServer()),
  );

  // TODO: I want a paste handler that translates HTML to RSX whenever HTML is pasted into an RSX
  // block, or a little tooltip that pops up and asks if you want to translate the HTML to RSX.
}

function formatOnSave(e: vscode.TextDocumentWillSaveEvent) {
  if (e.document.languageId !== 'rust') {
    return;
  }

  const dioxusConfig = vscode.workspace.getConfiguration('dioxus', e.document.uri);
  const dioxusFormatOnSave = dioxusConfig.get<string>('formatOnSave') ?? 'followFormatOnSave';

  if (dioxusFormatOnSave === 'disabled') {
    return;
  }

  const editorConfig = vscode.workspace.getConfiguration('editor', e.document.uri);
  const editorFormatOnSave = editorConfig.get<boolean>('formatOnSave') ?? false;

  if (dioxusFormatOnSave === 'followFormatOnSave' && !editorFormatOnSave) {
    return;
  }

  // Use `waitUntil` to properly synchronize with VSCode's save pipeline.
  e.waitUntil(formatDocument(e.document));
}

function formatDocument(document: vscode.TextDocument): Thenable<vscode.TextEdit[]> {
  return new Promise(resolve => {
    try {
      const contents = document.getText();

      const config = vscode.workspace.getConfiguration('editor', document.uri);

      const tabSize = config.get<number>('tabSize') ?? 4;
      const insertSpaces = config.get<boolean>('insertSpaces') ?? false;

      const formatted = dioxus.formatFile(contents, !insertSpaces, tabSize);

      if (formatted.length() > 0) {
        const fullRange = new vscode.Range(
          document.positionAt(0),
          document.positionAt(contents.length),
        );

        resolve([vscode.TextEdit.replace(fullRange, formatted.formatted())]);
      } else {
        resolve([]);
      }
    } catch (error) {
      if (error instanceof Error) {
        vscode.window.showErrorMessage(error.toString());
      } else {
        throw error;
      }

      resolve([]);
    }
  });
}

function translate(component: boolean) {
  const editor = vscode.window.activeTextEditor;

  if (!editor) {
    return;
  }

  const html = editor.document.getText(editor.selection);

  if (html.length == 0) {
    vscode.window.showWarningMessage('No HTML selected.');

    return;
  }

  const out = dioxus.translateRsx(html, component);

  if (out.length > 0) {
    editor.edit(editBuilder => {
      editBuilder.replace(editor.selection, out);
    });
  } else {
    vscode.window.showWarningMessage('Error translating. Make sure valid HTML is selected.');
  }
}

async function formatRsxDocument() {
  const editor = vscode.window.activeTextEditor;

  if (!editor) {
    return;
  }

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

  if (!editor) {
    return;
  }

  if (editor.document.languageId !== 'rust') {
    return;
  }

  let endLine = editor.selection.end.line;

  let selectionRange = new vscode.Range(
    editor.selection.start.line,
    0,
    endLine,
    editor.document.lineAt(endLine).range.end.character,
  );

  let unformatted = editor.document.getText(selectionRange);

  if (unformatted.trim().length == 0) {
    vscode.window.showWarningMessage('No RSX selected.');

    return;
  }

  while (
    (unformatted.match(/{/g) ?? []).length > (unformatted.match(/}/g) ?? []).length &&
    endLine < editor.document.lineCount - 1
  ) {
    ++endLine;

    selectionRange = new vscode.Range(
      editor.selection.start.line,
      0,
      endLine,
      editor.document.lineAt(endLine).range.end.character,
    );

    unformatted = editor.document.getText(selectionRange);
  }

  const tabSize = editor.options.tabSize as number;

  const endAbove = Math.max(editor.selection.start.line - 1, 0);

  const range = new vscode.Range(
    0,
    0,
    endAbove,
    editor.document.lineAt(endAbove).range.end.character,
  );

  const linesAbove = editor.document.getText(range);

  const openBraces = linesAbove.match(/{/g)?.length ?? 0;
  const closeBraces = linesAbove.match(/}/g)?.length ?? 0;

  const baseIndent = openBraces - closeBraces - 1;

  try {
    let formatted = dioxus.formatSelection(
      unformatted,
      !editor.options.insertSpaces,
      tabSize,
      baseIndent,
    );

    for (let i = 0; i <= baseIndent; ++i) {
      formatted = `${editor.options.insertSpaces ? ' '.repeat(tabSize) : '\t'}${formatted}`;
    }

    if (formatted.length > 0) {
      editor.edit(editBuilder => {
        editBuilder.replace(selectionRange, formatted);
      });
    }
  } catch (error) {
    if (error instanceof Error) {
      vscode.window.showErrorMessage(error.toString());
    } else {
      throw error;
    }
  }
}

class UriLaunchServer implements vscode.UriHandler {
  handleUri(uri: vscode.Uri): vscode.ProviderResult<void> {
    if (uri.path === '/debugger') {
      const query = decodeURIComponent(uri.query);
      const params = new URLSearchParams(query);
      const route = params.get('uri') ?? '';

      vscode.window.showInformationMessage(`Opening Chrome debugger: ${route}`);
      vscode.commands.executeCommand('extension.js-debug.debugLink', route);
    }
  }
}
