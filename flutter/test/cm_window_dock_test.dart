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
      if (call.method == 'setTitleBarStyle') return null;
      throw UnsupportedError(call.method);
    });
    addTearDown(() => messenger.setMockMethodCallHandler(channel, null));
    for (final option in ['Y', '']) {
      binding.option = option;
      await cmWindowDock.syncMinimizable();
      expect(await windowManager.isMinimizable(), option != 'Y');
    }
  });

  test(
      'opening chat or file transfers only widens compact CM',
      () async {
    const channel = MethodChannel('window_manager');
    var bounds = const Rect.fromLTWH(1884, 856, 36, 80);
    messenger.setMockMethodCallHandler(channel, (call) async {
      if (call.method == 'getBounds') {
        return {
          'x': bounds.left,
          'y': bounds.top,
          'width': bounds.width,
          'height': bounds.height
        };
      }
      if (call.method == 'setBounds') {
        final args = call.arguments;
        bounds =
            Rect.fromLTWH(args['x'], args['y'], args['width'], args['height']);
        return null;
      }
      if (call.method == 'setTitleBarStyle') return null;
      throw UnsupportedError(call.method);
    });
    addTearDown(() => messenger.setMockMethodCallHandler(channel, null));
    final dock = CmWindowDock()..value = true;
    addTearDown(dock.dispose);
    await dock.resizeExpanded(kConnectionManagerWindowSizeClosedChat);
    expect(await windowManager.getBounds(),
        const Rect.fromLTWH(1584, 680, 336, 320));
    dock.value = false;
    await dock.resizeExpanded(kConnectionManagerWindowSizeOpenChat);
    expect(await windowManager.getBounds(),
        const Rect.fromLTWH(1184, 680, 736, 320));
    await dock.resizeExpanded(kConnectionManagerWindowSizeClosedChat);
    expect(await windowManager.getBounds(),
        const Rect.fromLTWH(1584, 680, 336, 320));
  });

  testWidgets(
      'the app tab opens CM and the side arrow folds it without tooltips',
      (tester) async {
    var docked = true;
    await tester.pumpWidget(MaterialApp(
      home: StatefulBuilder(
        builder: (context, setState) => Align(
            alignment: Alignment.bottomRight,
            child: SizedBox.fromSize(
              size: docked ? CmWindowDock.size : const Size(336, 320),
              child: CmDockLayout(
                docked: docked,
                contentSize: CmWindowDock.compactSize,
                onToggle: () => setState(() => docked = !docked),
                child: const Material(child: TextField()),
              ),
            )),
      ),
    ));

    await tester.pumpAndSettle();
    expect(find.image(const AssetImage('assets/icon.png')), findsOneWidget);
    expect(find.byType(Tooltip), findsNothing);
    expect(tester.takeException(), isNull);
    expect(find.byType(TextField), findsNothing);
    await tester.tapAt(
        tester.getTopLeft(find.byType(CmDockTab)) + const Offset(18, 58));
    await tester.pumpAndSettle();
    expect(find.byIcon(Icons.chevron_right), findsOneWidget);
    expect(find.byType(TextField), findsOneWidget);
    expect(tester.getTopLeft(find.byType(TextField)).dx,
        tester.getTopLeft(find.byType(CmDockTab)).dx + 36);
    await tester.enterText(find.byType(TextField), 'Connection details');
    await tester.tap(find.byIcon(Icons.chevron_right));
    await tester.pumpAndSettle();
    expect(find.byType(TextField), findsNothing);
    await tester.tap(find.byType(CmDockTab));
    await tester.pumpAndSettle();
    expect(find.text('Connection details'), findsOneWidget);
    expect(find.byType(Tooltip), findsNothing);
    expect(tester.takeException(), isNull);
  });
}
