import * as vscode from 'vscode'

let currentPanel: vscode.WebviewPanel | undefined = undefined;

export function activate(context: vscode.ExtensionContext) {
    const disposable = vscode.commands.registerCommand('headless-cline.plusButtonClicked', () => {
        // WebViewパネルが既に存在する場合は、そのパネルを表示
        if (currentPanel) {
            currentPanel.reveal(vscode.ViewColumn.One);
            return;
        }

        // 新しいWebViewパネルを作成
        currentPanel = vscode.window.createWebviewPanel(
            'claudeChat',
            'Claude Chat',
            vscode.ViewColumn.One,
            {
                enableScripts: true
            }
        );

        // 初期HTMLを設定
        currentPanel.webview.html = getWebviewContent();

        // WebViewパネルが閉じられたときの処理
        currentPanel.onDidDispose(
            () => {
                currentPanel = undefined;
            },
            null,
            context.subscriptions
        );

        // Claude APIのストリーミングを開始
        import('../cline').then(async(module) => {
            let fullResponse = '';
            
            // コールバック関数を定義
            const updateContent = (text: string) => {
                // テキストを累積
                fullResponse += text;
                if (currentPanel) {
                    currentPanel.webview.html = getWebviewContent(fullResponse);
                }
                // デバッグログ
                console.log('Received update:', text);
            };

            try {
                await module.stream_response("こんにちは！", updateContent);
            } catch (error) {
                console.error('Error in stream_response:', error);
                if (currentPanel) {
                    currentPanel.webview.html = getWebviewContent(`Error: ${error}`);
                }
            }
        });
    });
    context.subscriptions.push(disposable)
}

function getWebviewContent(message = '') {
    return `<!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>Claude Chat</title>
        <style>
            body {
                padding: 20px;
                font-family: sans-serif;
                line-height: 1.6;
            }
            .message {
                margin: 10px 0;
                padding: 10px;
                background-color: #f0f0f0;
                border-radius: 5px;
                white-space: pre-wrap;
            }
        </style>
    </head>
    <body>
        <div class="message">
            ${message || 'Loading...'}
        </div>
    </body>
    </html>`;
}

export function deactivate() {
  return
}