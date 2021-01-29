/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */

import * as path from "path";
import {
  commands,
  CompletionList,
  ExtensionContext,
  Uri,
  workspace,
} from "vscode";
import { getLanguageService } from "vscode-html-languageservice";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient";
import { isInsideHtmlMacro } from "./rustSupport";
// import { getCSSVirtualContent, isInsideStyleRegion } from "./embeddedSupport";

let client: LanguageClient;

const htmlLanguageService = getLanguageService();

export function activate(context: ExtensionContext) {
  // The server is implemented in node
  let serverModule = context.asAbsolutePath(
    path.join("server", "out", "server.js")
  );
  // The debug options for the server
  // --inspect=6009: runs the server in Node's Inspector mode so VS Code can attach to the server for debugging
  let debugOptions = { execArgv: ["--nolazy", "--inspect=6009"] };

  // If the extension is launched in debug mode then the debug server options are used
  // Otherwise the run options are used
  let serverOptions: ServerOptions = {
    run: { module: serverModule, transport: TransportKind.ipc },
    debug: {
      module: serverModule,
      transport: TransportKind.ipc,
      options: debugOptions,
    },
  };

  const virtualDocumentContents = new Map<string, string>();

  workspace.registerTextDocumentContentProvider("embedded-content", {
    provideTextDocumentContent: (uri) => {
      const originalUri = uri.path.slice(1).slice(0, -4);
      console.error(originalUri);
      const decodedUri = decodeURIComponent(originalUri);
      return virtualDocumentContents.get(decodedUri);
    },
  });

  let clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "rust" }],
    middleware: {
      provideCompletionItem: async (
        document,
        position,
        context,
        token,
        next
      ) => {
        /*
				1: Find the occurences of the html! macro using regex
				2: Check if any of the occurences match the cursor offset
				3: If so, direct the captured block to the html to the rsx language service
        */
        const docSrc = document.getText();
        const offset = document.offsetAt(position);
        const matches = docSrc.matchAll(macroRegex);

        // Lazily loop through matches, abort early if the cursor is after the match
        // let start = 0;
        // let end = 0;
        let matchBody: string | undefined = undefined;

        for (const match of matches) {
          // // Check if the cursor is inbetween the previous end and the new start
          // // This means the cursor is between html! invocations and we should bail early
          // if (offset > end && offset < match.index) {
          //   // Ensure the match
          //   // defer to the "next?" symbol
          //   return await next(document, position, context, token);
          // }

          // Otherwise, move the counters forward
          const start = match.index;
          const end = start + match.length;

          // Ensure the cursor is within the match
          // Break if so
          if (offset >= start && offset <= end) {
            matchBody = match[1];
            break;
          }
        }

        // If we looped through all the matches and the match wasn't defined, then bail
        if (matchBody === undefined) {
          return await next(document, position, context, token);
        }

        // If we're inside the style region, then provide CSS completions with the CSS provider
        const originalUri = document.uri.toString();
        virtualDocumentContents.set(originalUri, matchBody);
        // getCSSVirtualContent(htmlLanguageService, document.getText())

        const vdocUriString = `embedded-content://html/${encodeURIComponent(
          originalUri
        )}.html`;

        const vdocUri = Uri.parse(vdocUriString);
        return await commands.executeCommand<CompletionList>(
          "vscode.executeCompletionItemProvider",
          vdocUri,
          position,
          context.triggerCharacter
        );
      },
    },
  };

  // Create the language client and start the client.
  client = new LanguageClient(
    "languageServerExample",
    "Language Server Example",
    serverOptions,
    clientOptions
  );
  // Start the client. This will also launch the server
  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

const macroRegex = /html! {([\s\S]*?)}/g;
