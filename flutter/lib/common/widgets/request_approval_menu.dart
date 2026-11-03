import 'dart:async';

import 'package:flutter/material.dart';

import '../../desktop/widgets/material_mod_popup_menu.dart' as mod_menu;
import '../../desktop/widgets/popup_menu.dart';

class RequestApprovalMenu extends MenuEntryBase<String> {
  final String title;
  final List<MenuEntryButton<String>> entries;
  final EdgeInsets? padding;
  final double arrowScale;

  RequestApprovalMenu(
      {required this.title,
      required this.entries,
      this.padding,
      this.arrowScale = 1.0});

  @override
  List<mod_menu.PopupMenuEntry<String>> build(
      BuildContext context, MenuConfig conf) {
    final theme = Theme.of(context);
    final checkboxTheme = CheckboxTheme.of(context);
    final checkboxWidth =
        ((checkboxTheme.materialTapTargetSize ?? theme.materialTapTargetSize) ==
                    MaterialTapTargetSize.padded
                ? kMinInteractiveDimension
                : kMinInteractiveDimension - 8) +
            (checkboxTheme.visualDensity ?? theme.visualDensity)
                .baseSizeAdjustment
                .dx;
    final arrowInset = (checkboxWidth - (IconTheme.of(context).size ?? 24)) / 2;
    return [
      _RequestApprovalItem(
        height: conf.height,
        padding: (padding ?? EdgeInsets.zero)
            .copyWith(right: (padding?.right ?? 0) + arrowInset),
        child: Row(children: [
          Text(title, style: enabledStyle(context)),
          Expanded(
            child: Align(
              alignment: Alignment.centerRight,
              child: Transform.scale(
                scale: arrowScale,
                child: Icon(Icons.keyboard_arrow_right,
                    color: theme.unselectedWidgetColor),
              ),
            ),
          ),
        ]),
        itemBuilder: (context) => entries
            .expand((entry) => MenuEntryButton<String>(
                  childBuilder: entry.childBuilder,
                  proc: entry.proc,
                  padding: entry.padding,
                  enabled: entry.enabled,
                  dismissOnClicked: true,
                  dismissCallback: () {
                    if (Navigator.canPop(context)) Navigator.pop(context);
                  },
                ).build(context, conf))
            .toList(),
      ),
    ];
  }
}

class _RequestApprovalItem extends mod_menu.PopupMenuEntry<String> {
  const _RequestApprovalItem({
    required this.height,
    required this.padding,
    required this.child,
    required this.itemBuilder,
  });

  @override
  final double height;
  final EdgeInsets padding;
  final Widget child;
  final mod_menu.PopupMenuItemBuilder<String> itemBuilder;

  @override
  bool represents(String? value) => false;

  @override
  State<_RequestApprovalItem> createState() => _RequestApprovalItemState();
}

class _RequestApprovalItemState extends State<_RequestApprovalItem> {
  Timer? _hoverTimer;
  bool _submenuOpen = false;

  @override
  void dispose() {
    _hoverTimer?.cancel();
    super.dispose();
  }

  void _hoverTrigger(bool hovered) {
    // Opening the submenu places its barrier over the trigger.
    if (_submenuOpen) return;
    _hoverTimer?.cancel();
    if (hovered) {
      _hoverTimer = Timer(const Duration(milliseconds: 200), _openSubmenu);
    }
  }

  Future<void> _openSubmenu() async {
    _hoverTimer?.cancel();
    if (!mounted || _submenuOpen || ModalRoute.of(context)?.isCurrent != true) {
      return;
    }
    final navigator = Navigator.of(context);
    final button = context.findRenderObject();
    final overlay = navigator.overlay?.context.findRenderObject();
    final items = widget.itemBuilder(context);
    if (button is! RenderBox || overlay is! RenderBox || items.isEmpty) return;
    final anchor =
        button.localToGlobal(Offset.zero, ancestor: overlay) & button.size;
    ModalRoute<dynamic>? submenuRoute;
    var dismissedByHover = false;

    void cancelClose() => _hoverTimer?.cancel();
    void scheduleClose() {
      if (_hoverTimer?.isActive == true) return;
      _hoverTimer = Timer(const Duration(milliseconds: 300), () {
        if (mounted && submenuRoute?.isCurrent == true) {
          dismissedByHover = true;
          navigator.pop();
        }
      });
    }

    void hoverBarrier(Offset position) {
      if (anchor.contains(overlay.globalToLocal(position))) {
        cancelClose();
      } else {
        scheduleClose();
      }
    }

    setState(() => _submenuOpen = true);
    await mod_menu.showMenu<String>(
      context: context,
      position: RelativeRect.fromRect(
          anchor.shift(Offset(anchor.width, 0)), Offset.zero & overlay.size),
      submenuAnchor: anchor,
      items: items,
      menuWrapper: (menu) => Builder(builder: (context) {
        submenuRoute = ModalRoute.of(context);
        return MouseRegion(
          onEnter: (_) => cancelClose(),
          onExit: (_) => scheduleClose(),
          child: menu,
        );
      }),
      barrierWrapper: (barrier) => MouseRegion(
        onEnter: (event) => hoverBarrier(event.position),
        onHover: (event) => hoverBarrier(event.position),
        onExit: (_) => scheduleClose(),
        child: barrier,
      ),
    );
    _hoverTimer?.cancel();
    if (!mounted) return;
    setState(() => _submenuOpen = false);
    if (!dismissedByHover && ModalRoute.of(context)?.isCurrent == true) {
      navigator.pop();
    }
  }

  @override
  Widget build(BuildContext context) {
    final highlight = Theme.of(context).colorScheme.onSurface.withOpacity(0.08);
    return MouseRegion(
      onEnter: (_) => _hoverTrigger(true),
      onExit: (_) => _hoverTrigger(false),
      child: TextButton(
        onPressed: _openSubmenu,
        style: ButtonStyle(
          shape: const MaterialStatePropertyAll(RoundedRectangleBorder()),
          backgroundColor: MaterialStatePropertyAll(
              _submenuOpen ? highlight : Colors.transparent),
          overlayColor: MaterialStateProperty.resolveWith((states) =>
              !_submenuOpen &&
                      (states.contains(MaterialState.hovered) ||
                          states.contains(MaterialState.focused) ||
                          states.contains(MaterialState.pressed))
                  ? highlight
                  : Colors.transparent),
        ),
        // Retain the old spacing, with one focus stop and one hover highlight.
        child: ExcludeFocus(
          child: TextButton(
            onPressed: _openSubmenu,
            style: const ButtonStyle(
              overlayColor: MaterialStatePropertyAll(Colors.transparent),
              splashFactory: NoSplash.splashFactory,
            ),
            child: Container(
              alignment: AlignmentDirectional.centerStart,
              height: widget.height,
              padding: widget.padding,
              child: widget.child,
            ),
          ),
        ),
      ),
    );
  }
}
