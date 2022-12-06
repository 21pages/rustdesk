import 'package:flutter/material.dart';

import 'package:flutter_hbb/common/widgets/peers_view.dart';
import 'package:flutter_hbb/desktop/widgets/login.dart';
import 'package:get/get.dart';

import '../../common.dart';
import '../../mobile/pages/settings_page.dart';

class MyDevices extends StatefulWidget {
  final EdgeInsets? menuPadding;
  const MyDevices({Key? key, this.menuPadding}) : super(key: key);

  @override
  State<StatefulWidget> createState() {
    return _MyDevicesState();
  }
}

class _MyDevicesState extends State<MyDevices> {
  var menuPos = RelativeRect.fill;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => gFFI.abModel.pullMd());
  }

  @override
  Widget build(BuildContext context) => FutureBuilder<Widget>(
      future: buildBody(context),
      builder: (context, snapshot) {
        if (snapshot.hasData) {
          return snapshot.data!;
        } else {
          return const Offstage();
        }
      });

  handleLogin() {
    if (isDesktop) {
      loginDialog().then((success) {
        if (success) {
          gFFI.abModel.pullAb();
        }
      });
    } else {
      showLogin(gFFI.dialogManager);
    }
  }

  Future<Widget> buildBody(BuildContext context) async {
    return Obx(() {
      if (gFFI.userModel.userName.value.isEmpty) {
        return Center(
          child: InkWell(
            onTap: handleLogin,
            child: Text(
              translate("Login"),
              style: const TextStyle(decoration: TextDecoration.underline),
            ),
          ),
        );
      } else {
        if (gFFI.abModel.abLoading.value) {
          return const Center(
            child: CircularProgressIndicator(),
          );
        }
        if (gFFI.abModel.abError.isNotEmpty) {
          return _buildShowError(gFFI.abModel.abError.value);
        }
        return Row(children: [_buildPeersViews()]);
      }
    });
  }

  Widget _buildShowError(String error) {
    return Center(
        child: Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        Text(translate(error)),
        TextButton(
            onPressed: () {
              gFFI.abModel.pullAb();
            },
            child: Text(translate("Retry")))
      ],
    ));
  }

  Widget _buildPeersViews() {
    return Expanded(
      child: Align(
          alignment: Alignment.topLeft,
          child: Obx(() => MyDevicesPeerView(
                menuPadding: widget.menuPadding,
                initPeers: gFFI.abModel.myDevices.value,
              ))),
    );
  }
}
