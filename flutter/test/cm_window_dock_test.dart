import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/consts.dart';
import 'package:flutter_hbb/desktop/widgets/cm_window_dock.dart';
import 'package:flutter_hbb/generated_bridge.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:window_manager/window_manager.dart';

class _Binding extends Fake implements RustdeskImpl {
  String option = '';

  @override
  String mainGetOptionSync({required String key, dynamic hint}) => option;

  @override
  String translate(
          {required String name, required String locale, dynamic hint}) =>
      name;
}

void main() {
  final messenger =
      TestWidgetsFlutterBinding.ensureInitialized().defaultBinaryMessenger;
  final binding = _Binding();

  setUp(() {
    binding.option = '';
    platformFFI.setBindingForTesting(binding);
  });

  test('CM docking is opt-in with Y', () async {
    final dock = CmWindowDock();
    addTearDown(dock.dispose);
    for (final option in ['', 'N', 'true']) {
      binding.option = option;
      expect(dock.enabled, isFalse);
      await dock.dock();
      expect(dock.value, isFalse);
    }
    binding.option = 'Y';
    expect(dock.enabled, isTrue);
    expect(option2bool(kOptionAllowDockCm, ''), isFalse);
    expect(option2bool(kOptionAllowDockCm, 'Y'), isTrue);
  });

  test('native minimization follows the docking option', () async {
    const channel = MethodChannel('window_manager');
    var minimizable = true;
    messenger.setMockMethodCallHandler(channel, (call) async {
      if (call.method == 'isMinimizable') return minimizable;
      if (call.method == 'setMinimizable') {
        minimizable = call.arguments['isMinimizable'] as bool;
        return null;
      }
      throw UnsupportedError(call.method);
    });
    addTearDown(() => messenger.setMockMethodCallHandler(channel, null));
    for (final option in ['Y', '']) {
      binding.option = option;
      await cmWindowDock.syncMinimizable();
      expect(await windowManager.isMinimizable(), option != 'Y');
    }
  });

  testWidgets('compact CM shows the app icon and expands without tooltips',
      (tester) async {
    var expanded = false;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: Center(
          child: SizedBox(
            width: CmWindowDock.size.width - 2,
            height: CmWindowDock.size.height - 2,
            child: CmDockIndicator(onExpand: () => expanded = true),
          ),
        ),
      ),
    ));

    await tester.pumpAndSettle();
    expect(find.image(const AssetImage('assets/icon.png')), findsOneWidget);
    expect(find.byType(Tooltip), findsNothing);
    expect(tester.takeException(), isNull);
    await tester.tap(find.byIcon(Icons.chevron_left));
    expect(expanded, isTrue);
  });
}
