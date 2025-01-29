import * as vscode from 'vscode'

export function activate(context: vscode.ExtensionContext) {
    // const disposable = vscode.commands.registerCommand(
    //     'headless-cline.plusButtonClicked',
    //     () => {
    //         vscode.window.showInformationMessage("aaaaa");

    //         import('../cline').then((module) => {
    //             const res = module.hellp_world()
    //             vscode.window.showInformationMessage(res);
    //         })

    //     }
    // )
    const disposable = vscode.commands.registerCommand('headless-cline.plusButtonClicked', () => {
		// The code you place here will be executed every time your command is executed
		// Display a message box to the user
        import('../cline').then(async(module) => {
            const res = await module.hellp_world()
            vscode.window.showInformationMessage(res);
        })
	});
    context.subscriptions.push(disposable)
}

export function deactivate() {
  return
}