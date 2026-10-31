import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';

import '../../common.dart';
import '../../consts.dart';
import '../../models/platform_model.dart';

final cmWindowDock = CmWindowDock();

class CmWindowDock extends ValueNotifier<bool> {
  CmWindowDock() : super(false);

  static const size = Size(36, 80);
  static const bottomInset = 64.0;
  static final compactSize =
      Size(kConnectionManagerWindowSizeClosedChat.width, 320);
  Future<void>? _transition;

  bool get enabled =>
      !kUseCompatibleUiMode &&
      bind.mainGetOptionSync(key: kOptionAllowDockCm) == 'Y';

  Size get expandedSize {
    final contentSize = gFFI.chatModel.isShowCMSidePage
        ? kConnectionManagerWindowSizeOpenChat
        : kConnectionManagerWindowSizeClosedChat;
    return enabled ? Size(contentSize.width, compactSize.height) : contentSize;
  }

  Future<void> syncMinimizable() async {
    final minimizable = !enabled;
    if (await windowManager.isMinimizable() != minimizable) {
      await windowManager.setMinimizable(minimizable);
      if (isMacOS) {
        await windowManager.setTitleBarStyle(TitleBarStyle.hidden,
            windowButtonVisibility: minimizable);
      }
    }
  }

  Future<void> resizeExpanded(Size contentSize, {bool align = false}) async {
    final windowSize = Size(contentSize.width + size.width, compactSize.height);
    if (isMacOS) {
      await windowManager.setTitleBarStyle(TitleBarStyle.hidden,
          windowButtonVisibility: false);
    }
    if (align) {
      await windowManager.setSizeAlignment(windowSize, Alignment.bottomRight);
    } else {
      final bounds = await windowManager.getBounds();
      final bottom = bounds.bottom + (value ? bottomInset : 0);
      await windowManager.setBounds(Rect.fromLTWH(
        bounds.right - windowSize.width,
        bottom - windowSize.height,
        windowSize.width,
        windowSize.height,
      ));
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
    final bounds = await windowManager.getBounds();
    if (isMacOS) {
      await windowManager.setTitleBarStyle(TitleBarStyle.hidden,
          windowButtonVisibility: false);
    }
    await windowManager.setBounds(Rect.fromLTWH(
      bounds.right - size.width,
      bounds.bottom - bottomInset - size.height,
      size.width,
      size.height,
    ));
    await windowManager.setAlwaysOnTop(true);
    await windowManager.show();
  }

  Future<void> expand() async {
    if (_transition != null) await _transition;
    if (!value) return;
    if (enabled) {
      await resizeExpanded(expandedSize);
    } else {
      await windowManager.setSizeAlignment(expandedSize, Alignment.topRight);
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
        return CmDockLayout(
          docked: docked,
          contentSize: cmWindowDock.expandedSize,
          onToggle: () async {
            if (docked) {
              await cmWindowDock.expand();
              await windowManager.focus();
            } else {
              await cmWindowDock.dock();
            }
          },
          child: child!,
        );
      },
    );
  }
}

class CmDockLayout extends StatelessWidget {
  const CmDockLayout({
    super.key,
    required this.docked,
    required this.contentSize,
    required this.onToggle,
    required this.child,
  });

  final bool docked;
  final Size contentSize;
  final VoidCallback onToggle;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
        builder: (context, constraints) => Stack(
              children: [
                Positioned(
                  left: docked ? 0 : CmWindowDock.size.width,
                  top: 0,
                  right: 0,
                  bottom: 0,
                  // Keep the connection controls mounted, at their normal size.
                  child: ExcludeFocus(
                    excluding: docked,
                    child: Offstage(
                      offstage: docked,
                      child: OverflowBox(
                        minWidth: docked ? contentSize.width : null,
                        maxWidth: docked ? contentSize.width : null,
                        minHeight: docked ? contentSize.height : null,
                        maxHeight: docked ? contentSize.height : null,
                        child: child,
                      ),
                    ),
                  ),
                ),
                Positioned(
                  left: 0,
                  bottom: constraints.maxHeight > CmWindowDock.size.height
                      ? CmWindowDock.bottomInset
                      : 0,
                  child: CmDockTab(docked: docked, onTap: onToggle),
                ),
              ],
            ));
  }
}

class CmDockTab extends StatelessWidget {
  const CmDockTab({super.key, required this.docked, required this.onTap});

  final bool docked;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return ClipPath(
      clipper: const _CmDockTabClipper(),
      child: Material(
        color: Colors.white,
        child: Semantics(
          button: true,
          label: translate(
              docked ? 'Restore' : 'Dock connection manager to the right'),
          child: InkWell(
            onTap: onTap,
            child: SizedBox.fromSize(
              size: CmWindowDock.size,
              child: Align(
                alignment: const Alignment(0, -0.35),
                child: docked
                    ? loadIcon(24)
                    : const Icon(Icons.chevron_right,
                        size: 24, color: Colors.blueGrey),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _CmDockTabClipper extends CustomClipper<Path> {
  const _CmDockTabClipper();

  @override
  Path getClip(Size size) => Path()
    ..moveTo(size.width, 0)
    ..lineTo(10, 0)
    ..quadraticBezierTo(0, 0, 0, 10)
    ..lineTo(0, size.height - 24)
    ..quadraticBezierTo(0, size.height - 18, 6, size.height - 15)
    ..lineTo(size.width, size.height)
    ..close();

  @override
  bool shouldReclip(_CmDockTabClipper oldClipper) => false;
}
