import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/desktop/widgets/tabbar_widget.dart';
import 'package:flutter_hbb/generated_bridge.dart';
import 'package:flutter_hbb/models/model.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:get/get.dart';
import 'package:uuid/uuid.dart';

class _Binding extends Fake implements RustdeskImpl {
  @override
  bool isIncomingOnly({dynamic hint}) => false;
  @override
  bool isOutgoingOnly({dynamic hint}) => false;
}

class _Session extends Fake implements FFI {
  @override
  final SessionID sessionId = const Uuid().v4obj();
}

void main() {
  for (final (tabType, hasOtherPeer) in [
    (DesktopTabType.fileTransfer, false),
    (DesktopTabType.fileTransfer, true),
    (DesktopTabType.portForward, true),
    (DesktopTabType.viewCamera, true),
  ]) {
    testWidgets(
        'close event for $tabType preserves another peer: $hasOtherPeer',
        (tester) async {
      platformFFI.setBindingForTesting(_Binding());
      final removed = <String>[];
      final controller = Get.put(DesktopTabController(
        tabType: tabType,
        onRemoved: (_, id) => removed.add(id),
      ));
      addTearDown(Get.reset);
      for (final id in ['peer-a', if (hasOtherPeer) 'peer-b']) {
        controller.state.value.tabs.add(
          TabInfo(key: id, label: id, page: const SizedBox()),
        );
      }
      controller.state.value.selected = hasOtherPeer ? 1 : 0;
      final session = _Session();
      final model = FfiModel(WeakReference(session));
      addTearDown(model.dispose);

      await model.startEventListener(session.sessionId, 'peer-a')(
        {'name': 'close_connection'},
      );

      expect(removed, ['peer-a']);
      expect(controller.length, hasOtherPeer ? 1 : 0);
      if (hasOtherPeer) {
        expect(controller.widget('peer-b'), isNotNull);
      }
      await tester.pump(const Duration(milliseconds: 100));
    });
  }

  testWidgets('terminal close event closes the peer window with all its tabs',
      (tester) async {
    final controller = Get.put(DesktopTabController(
      tabType: DesktopTabType.terminal,
      onRemoved: (_, __) => fail('Terminal connections close through the window'),
    ));
    addTearDown(Get.reset);
    for (final id in ['peer-a_1', 'peer-a_2']) {
      controller.state.value.tabs.add(
        TabInfo(key: id, label: id, page: const SizedBox()),
      );
    }
    var windowClosed = false;
    controller.onCloseWindow = () async {
      windowClosed = true;
      controller.clear();
    };
    final session = _Session();
    final model = FfiModel(WeakReference(session));
    addTearDown(model.dispose);

    await model.startEventListener(session.sessionId, 'peer-a')(
      {'name': 'close_connection'},
    );
    await tester.pump();

    expect(windowClosed, isTrue);
    expect(controller.length, 0);
  });
}
