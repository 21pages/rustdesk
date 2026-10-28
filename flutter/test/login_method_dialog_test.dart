import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/common/widgets/login_method_dialog.dart';
import 'package:flutter_hbb/common/widgets/dialog.dart';
import 'package:flutter_hbb/generated_bridge.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:uuid/uuid.dart';

class _Binding extends Fake implements RustdeskImpl {
  @override
  String translate(
          {required String name, required String locale, dynamic hint}) =>
      name;
}

Future<OverlayDialogManager> openDialog(
    WidgetTester tester, Map<String, dynamic> data) async {
  platformFFI.setBindingForTesting(_Binding());
  final overlay = OverlayKeyState();
  final manager = OverlayDialogManager()..setOverlayState(overlay);
  await tester.pumpWidget(MaterialApp(
    home: Overlay(
      key: overlay.key,
      initialEntries: [OverlayEntry(builder: (_) => const SizedBox())],
    ),
  ));
  showLoginMethodDialog(
      UuidValue('00000000-0000-4000-8000-000000000001'), manager, data);
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 200));
  return manager;
}

void main() {
  testWidgets('password-only peer explains the restriction and accepts input',
      (tester) async {
    final manager =
        await openDialog(tester, {'click': false, 'password': true});
    expect(find.text('Password Required'), findsOneWidget);
    expect(
        find.text(
            'The remote device only allows password connections. Enter the password to connect.'),
        findsOneWidget);
    expect(find.byType(PasswordWidget), findsOneWidget);
    expect(find.byType(CircularProgressIndicator), findsNothing);
    await tester.tap(find.text('Connect'));
    await tester.pump();
    expect(find.text('Empty Password'), findsOneWidget);
    await tester.enterText(find.byType(TextField), 'entered password');
    expect(find.text('entered password'), findsOneWidget);
    manager.dismissAll();
    await tester.pump();
  });

  testWidgets('click requests reuse the existing waiting dialog',
      (tester) async {
    for (final click in [true, null]) {
      final manager =
          await openDialog(tester, {'click': click, 'password': false});
      expect(find.text('Prompt'), findsOneWidget);
      expect(find.byIcon(Icons.hourglass_top), findsOneWidget);
      expect(
          find.text(click == null
              ? 'consent-method-unknown-tip'
              : 'Please wait for the remote side to accept your session request...'),
          findsOneWidget);
      expect(find.byType(CircularProgressIndicator), findsNothing);
      expect(find.byType(PasswordWidget), findsNothing);
      expect(find.text('Cancel'), findsOneWidget);
      expect(find.text('Connect'), findsNothing);
      manager.dismissAll();
      await tester.pump();
    }
  });
}
