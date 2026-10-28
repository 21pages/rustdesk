import 'package:flutter/material.dart';

import '../../desktop/widgets/material_mod_popup_menu.dart' as mod_menu;
import '../../desktop/widgets/popup_menu.dart';

class ConnectionMethodMenu extends MenuEntryBase<String> {
  final MenuEntryButton<String> defaultEntry;
  final List<MenuEntryButton<String>> methods;

  ConnectionMethodMenu({required this.defaultEntry, required this.methods});

  @override
  List<mod_menu.PopupMenuEntry<String>> build(
      BuildContext context, MenuConfig conf) {
    return MenuEntryButton<String>(
      padding: defaultEntry.padding,
      enabled: defaultEntry.enabled,
      dismissOnClicked: true,
      proc: defaultEntry.proc,
      childBuilder: (style) => Row(children: [
        Expanded(child: defaultEntry.childBuilder(style)),
        PopupMenuChildrenItem<String>(
          height: conf.height,
          padding: EdgeInsets.zero,
          enabled: defaultEntry.enabled,
          itemBuilder: (submenuContext) => methods
              .expand((entry) => MenuEntryButton<String>(
                    childBuilder: entry.childBuilder,
                    proc: entry.proc,
                    padding: entry.padding,
                    enabled: entry.enabled,
                    dismissOnClicked: true,
                    dismissCallback: () {
                      if (Navigator.canPop(context)) Navigator.pop(context);
                    },
                  ).build(submenuContext, conf))
              .toList(),
          child: Icon(Icons.keyboard_arrow_right, color: conf.commonColor),
        ),
      ]),
    ).build(context, conf);
  }
}
