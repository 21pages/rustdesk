import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/desktop/widgets/tabbar_widget.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:flutter_hbb/models/state_model.dart';
import 'package:get/get.dart';
import 'package:path/path.dart';
import 'package:url_launcher/url_launcher_string.dart';
import 'package:window_manager/window_manager.dart';

class InstallPage extends StatefulWidget {
  const InstallPage({Key? key}) : super(key: key);

  @override
  State<InstallPage> createState() => _InstallPageState();
}

class _InstallPageState extends State<InstallPage> {
  final tabController = DesktopTabController(tabType: DesktopTabType.main);

  @override
  void initState() {
    flog("install page initState");
    super.initState();
    Get.put<DesktopTabController>(tabController);
    const label = "install";
    tabController.add(TabInfo(
        key: label,
        label: label,
        closable: false,
        page: _InstallPageBody(
          key: const ValueKey(label),
        )));
  }

  @override
  void dispose() {
    super.dispose();
    Get.delete<DesktopTabController>();
  }

  @override
  Widget build(BuildContext context) {
    flog("install page build");
    return Container(
      child: Scaffold(
          backgroundColor: Theme.of(context).colorScheme.background,
          body: DesktopTab(controller: tabController)),
    );
  }
}

class _InstallPageBody extends StatefulWidget {
  const _InstallPageBody({Key? key}) : super(key: key);

  @override
  State<_InstallPageBody> createState() => _InstallPageBodyState();
}

class _InstallPageBodyState extends State<_InstallPageBody> {
  // late final TextEditingController controller;
  final RxBool startmenu = true.obs;
  final RxBool desktopicon = true.obs;
  final RxBool driverCert = true.obs;
  final RxBool showProgress = false.obs;
  final RxBool btnEnabled = true.obs;

  // todo move to theme.
  final buttonStyle = OutlinedButton.styleFrom(
    textStyle: TextStyle(fontSize: 14, fontWeight: FontWeight.normal),
    padding: EdgeInsets.symmetric(vertical: 15, horizontal: 12),
  );

  @override
  void initState() {
    flog("install body initState");
    // windowManager.addListener(this);
    // controller = TextEditingController(text: bind.installInstallPath());
    super.initState();
  }

  @override
  void dispose() {
    // windowManager.removeListener(this);
    super.dispose();
  }

  // @override
  // void onWindowClose() {
  //   gFFI.close();
  //   super.onWindowClose();
  //   windowManager.setPreventClose(false);
  //   windowManager.close();
  // }

  // InkWell Option(RxBool option, {String label = ''}) {
  //   return InkWell(
  //     // todo mouseCursor: "SystemMouseCursors.forbidden" or no cursor on btnEnabled == false
  //     borderRadius: BorderRadius.circular(6),
  //     onTap: () => btnEnabled.value ? option.value = !option.value : null,
  //     child: Row(
  //       children: [
  //         Obx(
  //           () => Checkbox(
  //             visualDensity: VisualDensity(horizontal: -4, vertical: -4),
  //             value: option.value,
  //             onChanged: (v) =>
  //                 btnEnabled.value ? option.value = !option.value : null,
  //           ).marginOnly(right: 8),
  //         ),
  //         Expanded(
  //           child: Text(translate(label)),
  //         ),
  //       ],
  //     ),
  //   );
  // }

  @override
  Widget build(BuildContext context) {
    flog("install body build");
    final double em = 13;
    // final isDarkTheme = MyTheme.currentThemeMode() == ThemeMode.dark;
    return Scaffold(
        backgroundColor: null, body: Text('You should see this text!'));
  }

  // void install() {
  //   do_install() {
  //     btnEnabled.value = false;
  //     showProgress.value = true;
  //     String args = '';
  //     if (startmenu.value) args += ' startmenu';
  //     if (desktopicon.value) args += ' desktopicon';
  //     if (driverCert.value) args += ' driverCert';
  //     bind.installInstallMe(options: args, path: controller.text);
  //   }

  //   if (driverCert.isTrue) {
  //     final tag = 'install-info-install-cert-confirm';
  //     final btns = [
  //       OutlinedButton.icon(
  //         icon: Icon(Icons.close_rounded, size: 16),
  //         label: Text(translate('Cancel')),
  //         onPressed: () => gFFI.dialogManager.dismissByTag(tag),
  //         style: buttonStyle,
  //       ),
  //       ElevatedButton.icon(
  //         icon: Icon(Icons.done_rounded, size: 16),
  //         label: Text(translate('OK')),
  //         onPressed: () {
  //           gFFI.dialogManager.dismissByTag(tag);
  //           do_install();
  //         },
  //         style: buttonStyle,
  //       )
  //     ];
  //     gFFI.dialogManager.show(
  //       (setState, close, context) => CustomAlertDialog(
  //         title: null,
  //         content: SelectionArea(
  //             child:
  //                 msgboxContent('info', 'Warning', 'confirm_install_cert_tip')),
  //         actions: btns,
  //         onCancel: close,
  //       ),
  //       tag: tag,
  //     );
  //   } else {
  //     do_install();
  //   }
  // }

  // void selectInstallPath() async {
  //   String? install_path = await FilePicker.platform
  //       .getDirectoryPath(initialDirectory: controller.text);
  //   if (install_path != null) {
  //     controller.text = join(install_path, await bind.mainGetAppName());
  //   }
  // }
}
