import 'package:flutter/material.dart';

import '../../common.dart';
import '../../models/platform_model.dart';
import 'dialog.dart';

void showConnectionMethodDialog(SessionID sessionId,
    OverlayDialogManager dialogManager, Map<String, dynamic> data) async {
  var method = data['method'] as String;
  final clickAllowed = data['click'] as bool?;
  final passwordAllowed = data['password'] as bool?;
  var error = data['error'] as String? ?? '';
  final controller = TextEditingController();
  dialogManager.dismissAll();
  await dialogManager.show((setState, close, context) {
    final consent = method == 'consent';
    final allowed = (consent ? clickAllowed : passwordAllowed) != false;

    cancel() {
      close();
      closeConnection();
    }

    submit() {
      if (consent || !allowed || controller.text.isEmpty) return;
      gFFI.login('', '', sessionId, controller.text, false);
      close();
      dialogManager.showLoading(translate('Logging in...'),
          onCancel: closeConnection);
    }

    switchMethod() {
      final next = consent ? 'password' : 'consent';
      if (!bind.sessionSetLoginMethod(sessionId: sessionId, method: next)) {
        cancel();
        return;
      }
      setState(() {
        method = next;
        error = '';
        controller.clear();
      });
      if (next == 'consent') {
        gFFI.login('', '', sessionId, '', false);
      }
    }

    final description = consent
        ? clickAllowed == false
            ? 'The remote device does not allow requests for consent.'
            : clickAllowed == null
                ? 'consent-method-unknown-tip'
                : 'Please wait for the remote side to accept your session request...'
        : passwordAllowed == false
            ? 'Password authentication is unavailable on the remote device.'
            : 'Saved passwords will not be used or changed.';
    return CustomAlertDialog(
      title: Row(children: [
        Icon(consent ? Icons.person_outline : Icons.lock_outline,
            color: MyTheme.accent),
        const SizedBox(width: 10),
        Flexible(
            child: Text(translate(consent
                ? clickAllowed == true
                    ? 'Waiting for approval'
                    : 'Request consent'
                : 'Enter password manually'))),
      ]),
      content: SizedBox(
        width: 360,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(translate(description)),
            if (consent && clickAllowed == true)
              const Padding(
                padding: EdgeInsets.only(top: 20),
                child: Center(child: CircularProgressIndicator()),
              ),
            if (!consent && allowed) ...[
              if (passwordAllowed == null)
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(translate('authentication-methods-unknown-tip')),
                ),
              const SizedBox(height: 16),
              PasswordWidget(controller: controller),
              if (error.isNotEmpty)
                Text(translate(error),
                    style:
                        TextStyle(color: Theme.of(context).colorScheme.error)),
            ],
            if ((consent ? passwordAllowed : clickAllowed) != false)
              TextButton(
                onPressed: switchMethod,
                child: Text(
                    translate(consent ? 'Use password' : 'Request consent')),
              ),
          ],
        ),
      ),
      actions: [
        dialogButton('Cancel', onPressed: cancel, isOutline: true),
        if (!consent && allowed) dialogButton('Connect', onPressed: submit),
      ],
      onSubmit: !consent && allowed ? submit : null,
      onCancel: cancel,
    );
  });
  controller.dispose();
}
