import 'package:debounce_throttle/debounce_throttle.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/consts.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:get/get.dart';

customImageQualityWidget(
    {required int initQuality,
    required Function(int) setQuality,
    required bool showMoreQuality}) {
  final maxVal = showMoreQuality ? kMaxMoreQuality : kMaxQuality;
  if (initQuality < kMinQuality || initQuality > maxVal) {
    initQuality = kDefaultQuality;
  }
  final qualityValue = initQuality.obs;

  final RxBool moreQualityChecked = RxBool(qualityValue.value > kMaxQuality);
  final debouncerQuality = Debouncer<double>(
    Duration(milliseconds: 1000),
    onChanged: (double v) {
      setQuality(v.round());
    },
    initialValue: qualityValue.value.toDouble(),
  );

  onMoreChanged(bool? value) {
    if (value == null) return;
    moreQualityChecked.value = value;
    if (!value && qualityValue.value > kMaxQuality) {
      qualityValue.value = kMaxQuality;
    }
    debouncerQuality.value = qualityValue.value.toDouble();
  }

  return Column(
    children: [
      Obx(() => Row(
            children: [
              Expanded(
                flex: 3,
                child: Slider(
                  value: qualityValue.value.toDouble(),
                  min: kMinQuality.toDouble(),
                  max: moreQualityChecked.value
                      ? kMaxMoreQuality.toDouble()
                      : kMaxQuality.toDouble(),
                  divisions: moreQualityChecked.value
                      ? ((kMaxMoreQuality - kMinQuality) / 5).round()
                      : ((kMaxQuality - kMinQuality) / 5).round(),
                  onChanged: (double value) async {
                    qualityValue.value = value.round();
                    debouncerQuality.value = value;
                  },
                ),
              ),
              Expanded(
                  flex: 1,
                  child: Text(
                    '${qualityValue.value.round()}%',
                    style: const TextStyle(fontSize: 15),
                  )),
              // mobile hide it for space and beauty
              if (isDesktop)
                Expanded(
                    flex: 1,
                    child: Text(
                      translate('Bitrate'),
                      style: const TextStyle(fontSize: 15),
                    )),
              if (showMoreQuality)
                Expanded(
                    flex: isMobile ? 3 : 1,
                    child: Row(
                      children: [
                        Checkbox(
                          value: moreQualityChecked.value,
                          onChanged: onMoreChanged,
                        ),
                        Expanded(
                          child: Text(translate('More')),
                        )
                      ],
                    ))
            ],
          )),
    ],
  );
}

customImageQualitySetting() {
  final qualityKey = 'custom_image_quality';
  var initQuality =
      (int.tryParse(bind.mainGetUserDefaultOption(key: qualityKey)) ??
          kDefaultQuality);
  return customImageQualityWidget(
      initQuality: initQuality,
      setQuality: (v) {
        bind.mainSetUserDefaultOption(key: qualityKey, value: v.toString());
      },
      showMoreQuality: true);
}

customFpsWidget({
  required RxInt rxFps,
  required Function(int) setFps,
}) {
  return ObxValue((p0) {
    return Row(
      children: [
        Expanded(
          flex: 4,
          child: Slider(
            value: p0.value.toDouble(),
            min: kMinFps.toDouble(),
            max: kMaxFps.toDouble(),
            divisions: ((kMaxFps - kMinFps) / 5).round(),
            onChanged: (double value) async {
              p0.value = value.round();
              await setFps(p0.value);
            },
          ),
        ),
        Expanded(
            flex: 1,
            child: Text(
              '${p0.value}',
              style: const TextStyle(fontSize: 15),
            )),
      ],
    );
  }, rxFps);
}

RxInt getSettingsInitialFps() {
  RxInt rxValue = kDefaultFps.obs;
  final fpsKey = 'custom-fps';
  int initFps =
      (int.tryParse(bind.mainGetUserDefaultOption(key: fpsKey)) ?? kDefaultFps);
  if (initFps < kMinFps || initFps > kMaxFps) {
    initFps = kDefaultFps;
  }
  rxValue.value = initFps;
  return rxValue;
}

customFpsSetting(RxInt rxFps) {
  return customFpsWidget(
    rxFps: rxFps,
    setFps: (v) async {
      final fpsKey = 'custom-fps';
      await bind.mainSetUserDefaultOption(key: fpsKey, value: v.toString());
    },
  );
}

Future<bool> setServerConfig(
  List<TextEditingController> controllers,
  List<RxString> errMsgs,
  ServerConfig config,
) async {
  config.idServer = config.idServer.trim();
  config.relayServer = config.relayServer.trim();
  config.apiServer = config.apiServer.trim();
  config.key = config.key.trim();
  // id
  if (config.idServer.isNotEmpty) {
    errMsgs[0].value =
        translate(await bind.mainTestIfValidServer(server: config.idServer));
    if (errMsgs[0].isNotEmpty) {
      return false;
    }
  }
  // relay
  if (config.relayServer.isNotEmpty) {
    errMsgs[1].value =
        translate(await bind.mainTestIfValidServer(server: config.relayServer));
    if (errMsgs[1].isNotEmpty) {
      return false;
    }
  }
  // api
  if (config.apiServer.isNotEmpty) {
    if (!config.apiServer.startsWith('http://') &&
        !config.apiServer.startsWith('https://')) {
      errMsgs[2].value =
          '${translate("API Server")}: ${translate("invalid_http")}';
      return false;
    }
  }
  final oldApiServer = await bind.mainGetApiServer();

  // should set one by one
  await bind.mainSetOption(
      key: 'custom-rendezvous-server', value: config.idServer);
  await bind.mainSetOption(key: 'relay-server', value: config.relayServer);
  await bind.mainSetOption(key: 'api-server', value: config.apiServer);
  await bind.mainSetOption(key: 'key', value: config.key);

  final newApiServer = await bind.mainGetApiServer();
  if (oldApiServer.isNotEmpty &&
      oldApiServer != newApiServer &&
      gFFI.userModel.isLogin) {
    gFFI.userModel.logOut(apiServer: oldApiServer);
  }
  return true;
}

List<Widget> ServerConfigImportExportWidgets(
  List<TextEditingController> controllers,
  List<RxString> errMsgs,
) {
  import() {
    Clipboard.getData(Clipboard.kTextPlain).then((value) {
      final text = value?.text;
      if (text != null && text.isNotEmpty) {
        try {
          final sc = ServerConfig.decode(text);
          if (sc.idServer.isNotEmpty) {
            controllers[0].text = sc.idServer;
            controllers[1].text = sc.relayServer;
            controllers[2].text = sc.apiServer;
            controllers[3].text = sc.key;
            Future<bool> success = setServerConfig(controllers, errMsgs, sc);
            success.then((value) {
              if (value) {
                showToast(
                    translate('Import server configuration successfully'));
              } else {
                showToast(translate('Invalid server configuration'));
              }
            });
          } else {
            showToast(translate('Invalid server configuration'));
          }
        } catch (e) {
          showToast(translate('Invalid server configuration'));
        }
      } else {
        showToast(translate('Clipboard is empty'));
      }
    });
  }

  export() {
    final text = ServerConfig(
            idServer: controllers[0].text.trim(),
            relayServer: controllers[1].text.trim(),
            apiServer: controllers[2].text.trim(),
            key: controllers[3].text.trim())
        .encode();
    debugPrint("ServerConfig export: $text");
    Clipboard.setData(ClipboardData(text: text));
    showToast(translate('Export server configuration successfully'));
  }

  return [
    Tooltip(
      message: translate('Import server config'),
      child: IconButton(
          icon: Icon(Icons.paste, color: Colors.grey), onPressed: import),
    ),
    Tooltip(
        message: translate('Export Server Config'),
        child: IconButton(
            icon: Icon(Icons.copy, color: Colors.grey), onPressed: export))
  ];
}
