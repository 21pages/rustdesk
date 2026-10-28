import 'package:flutter/material.dart';

import '../../common.dart';
import 'dialog.dart';

void showLoginMethodDialog(SessionID sessionId,
    OverlayDialogManager dialogManager, Map<String, dynamic> data) async {
  final clickAllowed = data['click'] as bool?;
  final passwordRequired = clickAllowed == false && data['password'] == true;
  final osAccountRequired = data['os_account_required'] == true;
  final error = data['error'] as String? ?? '';

  void showWaiting({String error = ''}) {
    var text = clickAllowed == null
        ? 'consent-method-unknown-tip'
        : 'Please wait for the remote side to accept your session request...';
    if (error.isNotEmpty) {
      text = '${translate(text)}\n\n${translate(error)}';
    }
    showWaitAcceptDialog(
        sessionId, 'wait-remote-accept-nook', 'Prompt', text, dialogManager);
  }

  if (clickAllowed != false && !osAccountRequired) {
    showWaiting(error: error);
    return;
  }

  final passwordController = TextEditingController();
  final osUsernameController = TextEditingController();
  final osPasswordController = TextEditingController();
  String? usernameError;
  String? passwordError;
  dialogManager.dismissAll();
  await dialogManager.show((setState, close, context) {
    final allowed = clickAllowed != false || passwordRequired;
    final canSubmit = allowed && (osAccountRequired || passwordRequired);

    cancel() {
      close();
      closeConnection();
    }

    submit() {
      if (!canSubmit) return;
      if (osAccountRequired && osUsernameController.text.trim().isEmpty) {
        setState(() => usernameError = translate('Empty Username'));
        return;
      }
      if (passwordRequired && passwordController.text.isEmpty) {
        setState(() => passwordError = translate('Empty Password'));
        return;
      }
      gFFI.login(
          osUsernameController.text.trim(),
          osPasswordController.text.trim(),
          sessionId,
          passwordController.text,
          false);
      close();
      if (passwordRequired) {
        dialogManager.showLoading(translate('Logging in...'),
            onCancel: closeConnection);
      } else {
        showWaiting();
      }
    }

    final description = passwordRequired
        ? 'The remote device only allows password connections. Enter the password to connect.'
        : clickAllowed == false
            ? 'Unable to connect. The remote device cannot accept sessions via click or password.'
            : 'terminal-admin-login-tip';
    return CustomAlertDialog(
      title: Row(children: [
        Icon(passwordRequired ? Icons.password_rounded : Icons.person_outline,
            color: MyTheme.accent),
        const SizedBox(width: 10),
        Flexible(
            child: Text(translate(passwordRequired
                ? 'Password Required'
                : 'Connect without password'))),
      ]),
      content: SizedBox(
        width: 360,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(translate(description)),
            if (osAccountRequired && allowed) ...[
              if (passwordRequired) Text(translate('terminal-admin-login-tip')),
              DialogTextField(
                title: translate(DialogTextField.kUsernameTitle),
                controller: osUsernameController,
                prefixIcon: DialogTextField.kUsernameIcon,
                errorText: usernameError,
              ),
              PasswordWidget(
                  controller: osPasswordController, autoFocus: false),
            ],
            if (passwordRequired) ...[
              if (osAccountRequired)
                Text(translate('verify_rustdesk_password_tip')),
              PasswordWidget(
                controller: passwordController,
                autoFocus: !osAccountRequired,
                errorText: passwordError,
              ),
            ],
            if (error.isNotEmpty)
              Text(translate(error),
                  style: TextStyle(color: Theme.of(context).colorScheme.error)),
          ],
        ),
      ),
      actions: [
        dialogButton('Cancel', onPressed: cancel, isOutline: true),
        if (canSubmit)
          dialogButton(
              passwordRequired ? 'Connect' : 'Connect without password',
              onPressed: submit),
      ],
      onSubmit: canSubmit ? submit : null,
      onCancel: cancel,
    );
  });
  osUsernameController.dispose();
  osPasswordController.dispose();
  passwordController.dispose();
}
