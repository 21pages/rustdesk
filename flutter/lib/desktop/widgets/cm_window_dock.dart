import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';

import '../../common.dart';
import '../../consts.dart';
import '../../models/platform_model.dart';

final cmWindowDock = CmWindowDock();

class CmWindowDock extends ValueNotifier<bool> {
  CmWindowDock() : super(false);

  static const size = Size(40, 80);
  Future<void>? _transition;

  bool get enabled =>
      !kUseCompatibleUiMode &&
      bind.mainGetOptionSync(key: kOptionAllowDockCm) == 'Y';

  Size get expandedSize => gFFI.chatModel.isShowCMSidePage
      ? kConnectionManagerWindowSizeOpenChat
      : kConnectionManagerWindowSizeClosedChat;

  Future<void> syncMinimizable() async {
    final minimizable = !enabled;
    if (await windowManager.isMinimizable() != minimizable) {
      await windowManager.setMinimizable(minimizable);
    }
  }

  Future<void> dock() async {
    if (!enabled || value || _transition != null) return;
    final model = gFFI.serverModel;
    final connected = model.clients.where((client) => !client.disconnected);
    if (model.hideCm ||
        connected.isEmpty ||
        connected.any((client) => !client.authorized)) return;
    model.cmHiddenTimer?.cancel();
    model.cmHiddenTimer = null;
    _transition = _dock();
    try {
      await _transition;
    } finally {
      _transition = null;
    }
  }

  Future<void> _dock() async {
    value = true;
    // Hide the full layout before the native window becomes smaller.
    await WidgetsBinding.instance.endOfFrame;
    if (isMacOS) {
      await windowManager.setTitleBarStyle(TitleBarStyle.hidden,
          windowButtonVisibility: false);
    }
    await windowManager.setSizeAlignment(size, Alignment.centerRight);
    await windowManager.setAlwaysOnTop(true);
    await windowManager.show();
  }

  Future<void> expand() async {
    if (_transition != null) await _transition;
    if (!value) return;
    await windowManager.setSizeAlignment(expandedSize, Alignment.topRight);
    if (isMacOS) {
      await windowManager.setTitleBarStyle(TitleBarStyle.hidden);
    }
    value = false;
  }
}

class CmWindowDockView extends StatelessWidget {
  const CmWindowDockView({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<bool>(
      valueListenable: cmWindowDock,
      child: child,
      builder: (context, docked, child) {
        if (!cmWindowDock.enabled && !docked) return child!;
        final size = cmWindowDock.expandedSize;
        return Stack(
          children: [
            // Keep the connection controls mounted, at their normal size.
            ExcludeFocus(
              excluding: docked,
              child: Offstage(
                offstage: docked,
                child: OverflowBox(
                  minWidth: docked ? size.width : null,
                  maxWidth: docked ? size.width : null,
                  minHeight: docked ? size.height : null,
                  maxHeight: docked ? size.height : null,
                  child: child,
                ),
              ),
            ),
            if (docked)
              Center(
                child: CmDockIndicator(
                  onExpand: () async {
                    await cmWindowDock.expand();
                    await windowManager.focus();
                  },
                ),
              ),
          ],
        );
      },
    );
  }
}

class CmDockIndicator extends StatelessWidget {
  const CmDockIndicator({super.key, required this.onExpand});

  final VoidCallback onExpand;

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        loadIcon(24),
        IconButton(
          padding: EdgeInsets.zero,
          constraints: const BoxConstraints.tightFor(width: 32, height: 32),
          icon: Icon(Icons.chevron_left, semanticLabel: translate('Restore')),
          onPressed: onExpand,
        ),
      ],
    );
  }
}
