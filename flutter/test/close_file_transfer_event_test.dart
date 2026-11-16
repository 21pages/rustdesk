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
  for (final hasOtherPeer in [false, true]) {
    testWidgets(
        'file transfer close event preserves another peer: $hasOtherPeer',
        (tester) async {
      platformFFI.setBindingForTesting(_Binding());
      final removed = <String>[];
      final controller = Get.put(DesktopTabController(
        tabType: DesktopTabType.fileTransfer,
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
        {'name': 'close_file_transfer'},
      );

      expect(removed, ['peer-a']);
      expect(controller.length, hasOtherPeer ? 1 : 0);
      if (hasOtherPeer) {
        expect(controller.widget('peer-b'), isNotNull);
      }
      await tester.pump(const Duration(milliseconds: 100));
    });
  }
}
