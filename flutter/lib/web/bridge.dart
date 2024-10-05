import 'dart:async';
import 'dart:js' as js;
import 'dart:convert';
import 'dart:typed_data';
import 'package:flutter/foundation.dart';
import 'package:uuid/uuid.dart';
import 'dart:html' as html;

import 'package:flutter_hbb/consts.dart';

final _privateConstructorUsedError = UnsupportedError(
    'It seems like you constructed your class using `MyClass._()`. This constructor is only meant to be used by freezed and you are not supposed to need it nor use it.\nPlease check the documentation here for more information: https://github.com/rrousselGit/freezed#adding-getters-and-methods-to-our-models');

mixin _$EventToUI {
  Object get field0 => throw _privateConstructorUsedError;
}

sealed class EventToUI {
  const factory EventToUI.event(
    String field0,
  ) = EventToUI_Event;
  const factory EventToUI.rgba(
    int field0,
  ) = EventToUI_Rgba;
  const factory EventToUI.texture(
    int field0,
    bool field1,
  ) = EventToUI_Texture;
}

class EventToUI_Event implements EventToUI {
  const EventToUI_Event(final String field0) : this.field = field0;
  final String field;
  String get field0 => field;
}

class EventToUI_Rgba implements EventToUI {
  const EventToUI_Rgba(final int field0) : field = field0;
  final int field;
  int get field0 => field;
}

class EventToUI_Texture implements EventToUI {
  const EventToUI_Texture(final int field0, final bool field1)
      : f0 = field0,
        f1 = field1;
  final int f0;
  final bool f1;
  int get field0 => f0;
  bool get field1 => f1;
}

class RustdeskImpl {
  Future<void> stopGlobalEventStream({required String appType, dynamic hint}) {
    throw UnimplementedError("1");
  }

  Future<void> hostStopSystemKeyPropagate(
      {required bool stopped, dynamic hint}) {
    throw UnimplementedError("2");
  }

  int peerGetDefaultSessionsCount({required String id, dynamic hint}) {
    return 0;
  }

  String sessionAddExistedSync(
      {required String id,
      required UuidValue sessionId,
      required Int32List displays,
      dynamic hint}) {
    return '';
  }

  String sessionAddSync(
      {required UuidValue sessionId,
      required String id,
      required bool isFileTransfer,
      required bool isPortForward,
      required bool isRdp,
      required String switchUuid,
      required bool forceRelay,
      required String password,
      required bool isSharedPassword,
      dynamic hint}) {
    return js.context.callMethod('setByName', [
      'session_add_sync',
      jsonEncode(
          {'id': id, 'password': password, 'isFileTransfer': isFileTransfer})
    ]);
  }

  Stream<EventToUI> sessionStart(
      {required UuidValue sessionId, required String id, dynamic hint}) {
    js.context.callMethod('setByName', [
      'session_start',
      jsonEncode({'id': id})
    ]);
    return Stream.empty();
  }

  Stream<EventToUI> sessionStartWithDisplays(
      {required UuidValue sessionId,
      required String id,
      required Int32List displays,
      dynamic hint}) {
    throw UnimplementedError("3");
  }

  Future<bool?> sessionGetRemember(
      {required UuidValue sessionId, dynamic hint}) {
    return Future(
        () => js.context.callMethod('getByName', ['remember']) == 'true');
  }

  Future<bool?> sessionGetToggleOption(
      {required UuidValue sessionId, required String arg, dynamic hint}) {
    return Future(
        () => sessionGetToggleOptionSync(sessionId: sessionId, arg: arg));
  }

  bool sessionGetToggleOptionSync(
      {required UuidValue sessionId, required String arg, dynamic hint}) {
    return 'true' == js.context.callMethod('getByName', ['option:toggle', arg]);
  }

  Future<String?> sessionGetOption(
      {required UuidValue sessionId, required String arg, dynamic hint}) {
    return Future(
        () => js.context.callMethod('getByName', ['option:session', arg]));
  }

  Future<void> sessionLogin(
      {required UuidValue sessionId,
      required String osUsername,
      required String osPassword,
      required String password,
      required bool remember,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'login',
          jsonEncode({
            'os_username': osUsername,
            'os_password': osPassword,
            'password': password,
            'remember': remember
          })
        ]));
  }

  Future<void> sessionSend2Fa(
      {required UuidValue sessionId,
      required String code,
      required bool trustThisDevice,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', ['send_2fa', code]));
  }

  Future<void> sessionClose({required UuidValue sessionId, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', ['session_close']));
  }

  Future<void> sessionRefresh(
      {required UuidValue sessionId, required int display, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', ['refresh']));
  }

  Future<void> sessionRecordScreen(
      {required UuidValue sessionId,
      required bool start,
      required int display,
      required int width,
      required int height,
      dynamic hint}) {
    throw UnimplementedError("4");
  }

  Future<void> sessionRecordStatus(
      {required UuidValue sessionId, required bool status, dynamic hint}) {
    throw UnimplementedError("5");
  }

  Future<void> sessionReconnect(
      {required UuidValue sessionId, required bool forceRelay, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', ['reconnect']));
  }

  Future<void> sessionToggleOption(
      {required UuidValue sessionId, required String value, dynamic hint}) {
    return Future(
        () => js.context.callMethod('setByName', ['option:toggle', value]));
  }

  Future<void> sessionTogglePrivacyMode(
      {required UuidValue sessionId,
      required String implKey,
      required bool on,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'toggle_privacy_mode',
          jsonEncode({'impl_key': implKey, 'on': on})
        ]));
  }

  Future<String?> sessionGetFlutterOption(
      {required UuidValue sessionId, required String k, dynamic hint}) {
    return Future(
        () => js.context.callMethod('getByName', ['option:flutter:peer', k]));
  }

  Future<void> sessionSetFlutterOption(
      {required UuidValue sessionId,
      required String k,
      required String v,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'option:flutter:peer',
          jsonEncode({'name': k, 'value': v})
        ]));
  }

  int getNextTextureKey({dynamic hint}) {
    return 0;
  }

  String getLocalFlutterOption({required String k, dynamic hint}) {
    return js.context.callMethod('getByName', ['option:flutter:local', k]);
  }

  Future<void> setLocalFlutterOption(
      {required String k, required String v, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'option:flutter:local',
          jsonEncode({'name': k, 'value': v})
        ]));
  }

  String getLocalKbLayoutType({dynamic hint}) {
    return js.context.callMethod('getByName', ['option:local', 'kb_layout']);
  }

  Future<void> setLocalKbLayoutType(
      {required String kbLayoutType, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'option:local',
          jsonEncode({'name': 'kb_layout', 'value': kbLayoutType})
        ]));
  }

  Future<String?> sessionGetViewStyle(
      {required UuidValue sessionId, dynamic hint}) {
    return Future(() =>
        js.context.callMethod('getByName', ['option:session', 'view_style']));
  }

  Future<void> sessionSetViewStyle(
      {required UuidValue sessionId, required String value, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'option:session',
          jsonEncode({'name': 'view_style', 'value': value})
        ]));
  }

  Future<String?> sessionGetScrollStyle(
      {required UuidValue sessionId, dynamic hint}) {
    return Future(() =>
        js.context.callMethod('getByName', ['option:session', 'scroll_style']));
  }

  Future<void> sessionSetScrollStyle(
      {required UuidValue sessionId, required String value, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'option:session',
          jsonEncode({'name': 'scroll_style', 'value': value})
        ]));
  }

  Future<String?> sessionGetImageQuality(
      {required UuidValue sessionId, dynamic hint}) {
    return Future(() => js.context
        .callMethod('getByName', ['option:session', 'image_quality']));
  }

  Future<void> sessionSetImageQuality(
      {required UuidValue sessionId, required String value, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'option:session',
          jsonEncode({'name': 'image_quality', 'value': value})
        ]));
  }

  Future<String?> sessionGetKeyboardMode(
      {required UuidValue sessionId, dynamic hint}) {
    final mode =
        js.context.callMethod('getByName', ['option:session', 'keyboard_mode']);
    return Future(() => mode == '' ? null : mode);
  }

  Future<void> sessionSetKeyboardMode(
      {required UuidValue sessionId, required String value, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'option:session',
          jsonEncode({'name': 'keyboard_mode', 'value': value})
        ]));
  }

  String? sessionGetReverseMouseWheelSync(
      {required UuidValue sessionId, dynamic hint}) {
    return js.context
        .callMethod('getByName', ['option:session', 'reverse_mouse_wheel']);
  }

  Future<void> sessionSetReverseMouseWheel(
      {required UuidValue sessionId, required String value, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'option:session',
          jsonEncode({'name': 'reverse_mouse_wheel', 'value': value})
        ]));
  }

  String? sessionGetDisplaysAsIndividualWindows(
      {required UuidValue sessionId, dynamic hint}) {
    return js.context.callMethod(
        'getByName', ['option:session', 'displays_as_individual_windows']);
  }

  Future<void> sessionSetDisplaysAsIndividualWindows(
      {required UuidValue sessionId, required String value, dynamic hint}) {
    return Future.value();
  }

  String? sessionGetUseAllMyDisplaysForTheRemoteSession(
      {required UuidValue sessionId, dynamic hint}) {
    return '';
  }

  Future<void> sessionSetUseAllMyDisplaysForTheRemoteSession(
      {required UuidValue sessionId, required String value, dynamic hint}) {
    return Future.value();
  }

  Future<Int32List?> sessionGetCustomImageQuality(
      {required UuidValue sessionId, dynamic hint}) {
    try {
      return Future(() => Int32List.fromList([
            int.parse(js.context.callMethod(
                'getByName', ['option:session', 'custom_image_quality']))
          ]));
    } catch (e) {
      return Future.value(null);
    }
  }

  bool sessionIsKeyboardModeSupported(
      {required UuidValue sessionId, required String mode, dynamic hint}) {
    if (mainGetInputSource(hint: hint) == 'Input source 1') {
      return [kKeyMapMode, kKeyTranslateMode].contains(mode);
    } else {
      return [kKeyLegacyMode, kKeyMapMode].contains(mode);
    }
  }

  bool sessionIsMultiUiSession({required UuidValue sessionId, dynamic hint}) {
    return false;
  }

  Future<void> sessionSetCustomImageQuality(
      {required UuidValue sessionId, required int value, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'option:session',
          jsonEncode({'name': 'custom_image_quality', 'value': value})
        ]));
  }

  Future<void> sessionSetCustomFps(
      {required UuidValue sessionId, required int fps, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'option:session',
          jsonEncode({'name': 'custom_fps', 'value': fps})
        ]));
  }

  Future<void> sessionLockScreen({required UuidValue sessionId, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', ['lock_screen']));
  }

  Future<void> sessionCtrlAltDel({required UuidValue sessionId, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', ['ctrl_alt_del']));
  }

  Future<void> sessionSwitchDisplay(
      {required bool isDesktop,
      required UuidValue sessionId,
      required Int32List value,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'switch_display',
          jsonEncode({
            'isDesktop': isDesktop,
            'sessionId': sessionId.toString(),
            'value': value
          })
        ]));
  }

  Future<void> sessionHandleFlutterKeyEvent(
      {required UuidValue sessionId,
      required String character,
      required int usbHid,
      required int lockModes,
      required bool downOrUp,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'flutter_key_event',
          jsonEncode({
            'name': character,
            'usb_hid': usbHid,
            'lock_modes': lockModes,
            if (downOrUp) 'down': 'true',
          })
        ]));
  }

  Future<void> sessionHandleFlutterRawKeyEvent(
      {required UuidValue sessionId,
      required String name,
      required int platformCode,
      required int positionCode,
      required int lockModes,
      required bool downOrUp,
      dynamic hint}) {
    throw UnimplementedError("6");
  }

  void sessionEnterOrLeave(
      {required UuidValue sessionId, required bool enter, dynamic hint}) {
    js.context.callMethod('setByName', ['enter_or_leave', enter]);
  }

  Future<void> sessionInputKey(
      {required UuidValue sessionId,
      required String name,
      required bool down,
      required bool press,
      required bool alt,
      required bool ctrl,
      required bool shift,
      required bool command,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'input_key',
          jsonEncode({
            'name': name,
            if (down) 'down': 'true',
            if (press) 'press': 'true',
            if (alt) 'alt': 'true',
            if (ctrl) 'ctrl': 'true',
            if (shift) 'shift': 'true',
            if (command) 'command': 'true'
          })
        ]));
  }

  Future<void> sessionInputString(
      {required UuidValue sessionId, required String value, dynamic hint}) {
    return Future(
        () => js.context.callMethod('setByName', ['input_string', value]));
  }

  Future<void> sessionSendChat(
      {required UuidValue sessionId, required String text, dynamic hint}) {
    throw UnimplementedError("7");
  }

  Future<void> sessionPeerOption(
      {required UuidValue sessionId,
      required String name,
      required String value,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'option:session',
          jsonEncode({'name': name, 'value': value})
        ]));
  }

  Future<String> sessionGetPeerOption(
      {required UuidValue sessionId, required String name, dynamic hint}) {
    return Future(
        () => js.context.callMethod('getByName', ['option:session', name]));
  }

  Future<void> sessionInputOsPassword(
      {required UuidValue sessionId, required String value, dynamic hint}) {
    return Future(
        () => js.context.callMethod('setByName', ['input_os_password', value]));
  }

  Future<void> sessionReadRemoteDir(
      {required UuidValue sessionId,
      required String path,
      required bool includeHidden,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'read_remote_dir',
          jsonEncode({'path': path, 'include_hidden': includeHidden})
        ]));
  }

  Future<void> sessionSendFiles(
      {required UuidValue sessionId,
      required int actId,
      required String path,
      required String to,
      required int fileNum,
      required bool includeHidden,
      required bool isRemote,
      required bool isDir,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'send_files',
          jsonEncode({
            'id': actId,
            'path': path,
            'to': to,
            'file_num': fileNum,
            'include_hidden': includeHidden,
            'is_remote': isRemote,
            'is_dir': isDir,
          })
        ]));
  }

  Future<void> sessionSetConfirmOverrideFile(
      {required UuidValue sessionId,
      required int actId,
      required int fileNum,
      required bool needOverride,
      required bool remember,
      required bool isUpload,
      dynamic hint}) {
    throw UnimplementedError("10");
  }

  Future<void> sessionRemoveFile(
      {required UuidValue sessionId,
      required int actId,
      required String path,
      required int fileNum,
      required bool isRemote,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'remove_file',
          jsonEncode({
            'id': actId,
            'path': path,
            'file_num': fileNum,
            'is_remote': isRemote
          })
        ]));
  }

  Future<void> sessionReadDirToRemoveRecursive(
      {required UuidValue sessionId,
      required int actId,
      required String path,
      required bool isRemote,
      required bool showHidden,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'read_dir_to_remove_recursive',
          jsonEncode({
            'id': actId,
            'path': path,
            'is_remote': isRemote,
            'show_hidden': showHidden
          })
        ]));
  }

  Future<void> sessionRemoveAllEmptyDirs(
      {required UuidValue sessionId,
      required int actId,
      required String path,
      required bool isRemote,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'remove_all_empty_dirs',
          jsonEncode({'id': actId, 'path': path, 'is_remote': isRemote})
        ]));
  }

  Future<void> sessionCancelJob(
      {required UuidValue sessionId, required int actId, dynamic hint}) {
    return Future(
        () => js.context.callMethod('setByName', ['cancel_job', actId]));
  }

  Future<void> sessionCreateDir(
      {required UuidValue sessionId,
      required int actId,
      required String path,
      required bool isRemote,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'create_dir',
          jsonEncode({'id': actId, 'path': path, 'is_remote': isRemote})
        ]));
  }

  Future<String> sessionReadLocalDirSync(
      {required UuidValue sessionId,
      required String path,
      required bool showHidden,
      dynamic hint}) {
    throw UnimplementedError("16");
  }

  Future<String> sessionGetPlatform(
      {required UuidValue sessionId, required bool isRemote, dynamic hint}) {
    if (isRemote) {
      return Future(() => js.context.callMethod('getByName', ['platform']));
    } else {
      return Future(() => 'Web');
    }
  }

  Future<void> sessionLoadLastTransferJobs(
      {required UuidValue sessionId, dynamic hint}) {
    throw UnimplementedError("18");
  }

  Future<void> sessionAddJob(
      {required UuidValue sessionId,
      required int actId,
      required String path,
      required String to,
      required int fileNum,
      required bool includeHidden,
      required bool isRemote,
      dynamic hint}) {
    throw UnimplementedError("19");
  }

  Future<void> sessionResumeJob(
      {required UuidValue sessionId,
      required int actId,
      required bool isRemote,
      dynamic hint}) {
    throw UnimplementedError("20");
  }

  Future<void> sessionElevateDirect(
      {required UuidValue sessionId, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', ['elevate_direct']));
  }

  Future<void> sessionElevateWithLogon(
      {required UuidValue sessionId,
      required String username,
      required String password,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'elevate_with_logon',
          jsonEncode({'username': username, 'password': password})
        ]));
  }

  Future<void> sessionSwitchSides(
      {required UuidValue sessionId, dynamic hint}) {
    throw UnimplementedError("21");
  }

  Future<void> sessionChangeResolution(
      {required UuidValue sessionId,
      required int display,
      required int width,
      required int height,
      dynamic hint}) {
    // note: restore on disconnected
    throw UnimplementedError("22");
  }

  Future<void> sessionSetSize(
      {required UuidValue sessionId,
      required int display,
      required int width,
      required int height,
      dynamic hint}) {
    return Future.value();
  }

  Future<void> sessionSendSelectedSessionId(
      {required UuidValue sessionId, required String sid, dynamic hint}) {
    throw UnimplementedError("23");
  }

  Future<List<String>> mainGetSoundInputs({dynamic hint}) {
    throw UnimplementedError("24");
  }

  Future<String?> mainGetDefaultSoundInput({dynamic hint}) {
    throw UnimplementedError("25");
  }

  String mainGetLoginDeviceInfo({dynamic hint}) {
    String userAgent = html.window.navigator.userAgent;
    String appName = html.window.navigator.appName;
    String appVersion = html.window.navigator.appVersion;
    String? platform = html.window.navigator.platform;
    return jsonEncode({
      'os': '$userAgent, $appName $appVersion ($platform)',
      'type': 'Web client',
      'name': js.context.callMethod('getByName', ['my_name']),
    });
  }

  Future<void> mainChangeId({required String newId, dynamic hint}) {
    throw UnimplementedError("26");
  }

  Future<String> mainGetAsyncStatus({dynamic hint}) {
    throw UnimplementedError("27");
  }

  Future<String> mainGetOption({required String key, dynamic hint}) {
    return Future.value(mainGetOptionSync(key: key));
  }

  String mainGetOptionSync({required String key, dynamic hint}) {
    return js.context.callMethod('getByName', ['option', key]);
  }

  Future<String> mainGetError({dynamic hint}) {
    throw UnimplementedError("28");
  }

  bool mainShowOption({required String key, dynamic hint}) {
    throw UnimplementedError("29");
  }

  Future<void> mainSetOption(
      {required String key, required String value, dynamic hint}) {
    js.context.callMethod('setByName', [
      'option',
      jsonEncode({'name': key, 'value': value})
    ]);
    return Future.value();
  }

  // get server settings
  Future<String> mainGetOptions({dynamic hint}) {
    return Future(() => mainGetOptionsSync());
  }

  // get server settings
  String mainGetOptionsSync({dynamic hint}) {
    return js.context.callMethod('getByName', ['options']);
  }

  Future<void> mainSetOptions({required String json, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', ['options', json]));
  }

  Future<String> mainTestIfValidServer(
      {required String server, required bool testWithProxy, dynamic hint}) {
    // TODO: implement
    return Future.value('');
  }

  Future<void> mainSetSocks(
      {required String proxy,
      required String username,
      required String password,
      dynamic hint}) {
    throw UnimplementedError("30");
  }

  Future<List<String>> mainGetSocks({dynamic hint}) {
    throw UnimplementedError("31");
  }

  Future<String> mainGetAppName({dynamic hint}) {
    return Future.value(mainGetAppNameSync(hint: hint));
  }

  String mainGetAppNameSync({dynamic hint}) {
    return 'RustDesk';
  }

  String mainUriPrefixSync({dynamic hint}) {
    throw UnimplementedError("32");
  }

  Future<String> mainGetLicense({dynamic hint}) {
    // TODO: implement
    return Future(() => '');
  }

  Future<String> mainGetVersion({dynamic hint}) {
    return Future(() => js.context.callMethod('getByName', ['version']));
  }

  Future<List<String>> mainGetFav({dynamic hint}) {
    List<String> favs = [];
    try {
      favs = (jsonDecode(js.context.callMethod('getByName', ['fav']))
              as List<dynamic>)
          .map((e) => e.toString())
          .toList();
    } catch (e) {
      debugPrint('Failed to load favs: $e');
    }
    return Future.value(favs);
  }

  Future<void> mainStoreFav({required List<String> favs, dynamic hint}) {
    return Future(
        () => js.context.callMethod('setByName', ['fav', jsonEncode(favs)]));
  }

  String mainGetPeerSync({required String id, dynamic hint}) {
    // TODO:
    throw UnimplementedError("33");
  }

  Future<String> mainGetLanPeers({dynamic hint}) {
    throw UnimplementedError("34");
  }

  Future<String> mainGetConnectStatus({dynamic hint}) {
    return Future(
        () => js.context.callMethod('getByName', ["get_conn_status"]));
  }

  Future<void> mainCheckConnectStatus({dynamic hint}) {
    throw UnimplementedError("35");
  }

  Future<bool> mainIsUsingPublicServer({dynamic hint}) {
    return Future(() =>
        js.context.callMethod('getByName', ["is_using_public_server"]) ==
        'true');
  }

  Future<void> mainDiscover({dynamic hint}) {
    throw UnimplementedError("36");
  }

  Future<String> mainGetApiServer({dynamic hint}) {
    return Future(() => js.context.callMethod('getByName', ['api_server']));
  }

  Future<void> mainPostRequest(
      {required String url,
      required String body,
      required String header,
      dynamic hint}) {
    throw UnimplementedError("37");
  }

  Future<bool> mainGetProxyStatus({dynamic hint}) {
    return Future(() => false);
  }

  Future<void> mainHttpRequest({
    required String url,
    required String method,
    String? body,
    required String header,
    dynamic hint,
  }) {
    throw UnimplementedError("38");
  }

  Future<String?> mainGetHttpStatus({required String url, dynamic hint}) {
    throw UnimplementedError("39");
  }

  String mainGetLocalOption({required String key, dynamic hint}) {
    return js.context.callMethod('getByName', ['option:local', key]);
  }

  String mainGetEnv({required String key, dynamic hint}) {
    throw UnimplementedError("40");
  }

  Future<void> mainSetLocalOption(
      {required String key, required String value, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'option:local',
          jsonEncode({'name': key, 'value': value})
        ]));
  }

  String mainGetInputSource({dynamic hint}) {
    final inputSource =
        js.context.callMethod('getByName', ['option:local', 'input-source']);
    // // js grab mode
    // export const CONFIG_INPUT_SOURCE_1 = "Input source 1";
    // // flutter grab mode
    // export const CONFIG_INPUT_SOURCE_2 = "Input source 2";
    return inputSource != '' ? inputSource : 'Input source 1';
  }

  Future<void> mainSetInputSource(
      {required UuidValue sessionId, required String value, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'option:local',
          jsonEncode({'name': 'input-source', 'value': value})
        ]));
  }

  Future<String> mainGetMyId({dynamic hint}) {
    return Future(() => js.context.callMethod('getByName', ['my_id']));
  }

  Future<String> mainGetUuid({dynamic hint}) {
    return Future(() => js.context.callMethod('getByName', ['uuid']));
  }

  Future<String> mainGetPeerOption(
      {required String id, required String key, dynamic hint}) {
    return Future(() => mainGetPeerOptionSync(id: id, key: key, hint: hint));
  }

  String mainGetPeerOptionSync(
      {required String id, required String key, dynamic hint}) {
    return js.context.callMethod('getByName', [
      'option:peer',
      jsonEncode({'id': id, 'name': key})
    ]);
  }

  String mainGetPeerFlutterOptionSync(
      {required String id, required String k, dynamic hint}) {
    return js.context.callMethod('getByName', ['option:flutter:peer', k]);
  }

  void mainSetPeerFlutterOptionSync(
      {required String id,
      required String k,
      required String v,
      dynamic hint}) {
    js.context.callMethod('setByName', [
      'option:flutter:peer',
      jsonEncode({'name': k, 'value': v})
    ]);
  }

  Future<void> mainSetPeerOption(
      {required String id,
      required String key,
      required String value,
      dynamic hint}) {
    mainSetPeerOptionSync(id: id, key: key, value: value, hint: hint);
    return Future.value();
  }

  bool mainSetPeerOptionSync(
      {required String id,
      required String key,
      required String value,
      dynamic hint}) {
    js.context.callMethod('setByName', [
      'option:peer',
      jsonEncode({'id': id, 'name': key, 'value': value})
    ]);
    return true;
  }

  Future<void> mainSetPeerAlias(
      {required String id, required String alias, dynamic hint}) {
    mainSetPeerOptionSync(id: id, key: 'alias', value: alias, hint: hint);
    return Future.value();
  }

  Future<String> mainGetNewStoredPeers({dynamic hint}) {
    throw UnimplementedError("41");
  }

  Future<void> mainForgetPassword({required String id, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', ['forget']));
  }

  Future<bool> mainPeerHasPassword({required String id, dynamic hint}) {
    return Future(() =>
        js.context.callMethod('getByName', ['peer_has_password', id]) ==
        'true');
  }

  Future<bool> mainPeerExists({required String id, dynamic hint}) {
    return Future(
        () => js.context.callMethod('getByName', ['peer_exists', id]));
  }

  Future<void> mainLoadRecentPeers({dynamic hint}) {
    return Future(
        () => js.context.callMethod('getByName', ['load_recent_peers']));
  }

  String mainLoadRecentPeersSync({dynamic hint}) {
    return js.context.callMethod('getByName', ['load_recent_peers_sync']);
  }

  String mainLoadLanPeersSync({dynamic hint}) {
    return '{}';
  }

  Future<String> mainLoadRecentPeersForAb(
      {required String filter, dynamic hint}) {
    throw UnimplementedError("42");
  }

  Future<void> mainLoadFavPeers({dynamic hint}) {
    return Future(() => js.context.callMethod('getByName', ['load_fav_peers']));
  }

  Future<void> mainLoadLanPeers({dynamic hint}) {
    throw UnimplementedError("43");
  }

  Future<void> mainRemoveDiscovered({required String id, dynamic hint}) {
    throw UnimplementedError("44");
  }

  Future<void> mainChangeTheme({required String dark, dynamic hint}) {
    throw UnimplementedError("45");
  }

  Future<void> mainChangeLanguage({required String lang, dynamic hint}) {
    throw UnimplementedError("46");
  }

  String mainVideoSaveDirectory({required bool root, dynamic hint}) {
    throw UnimplementedError("47");
  }

  Future<void> mainSetUserDefaultOption(
      {required String key, required String value, dynamic hint}) {
    js.context.callMethod('setByName', [
      'option:user:default',
      jsonEncode({'name': key, 'value': value})
    ]);
    return Future.value();
  }

  String mainGetUserDefaultOption({required String key, dynamic hint}) {
    return js.context.callMethod('getByName', ['option:user:default', key]);
  }

  Future<String> mainHandleRelayId({required String id, dynamic hint}) {
    var newId = id;
    if (id.endsWith("\\r") || id.endsWith("/r")) {
      newId = id.substring(0, id.length - 2);
    }
    return Future.value(newId);
  }

  String mainGetMainDisplay({dynamic hint}) {
    return js.context.callMethod('getByName', ['main_display']);
  }

  String mainGetDisplays({dynamic hint}) {
    throw UnimplementedError("48");
  }

  Future<void> sessionAddPortForward(
      {required UuidValue sessionId,
      required int localPort,
      required String remoteHost,
      required int remotePort,
      dynamic hint}) {
    throw UnimplementedError("49");
  }

  Future<void> sessionRemovePortForward(
      {required UuidValue sessionId, required int localPort, dynamic hint}) {
    throw UnimplementedError("50");
  }

  Future<void> sessionNewRdp({required UuidValue sessionId, dynamic hint}) {
    throw UnimplementedError("51");
  }

  Future<void> sessionRequestVoiceCall(
      {required UuidValue sessionId, dynamic hint}) {
    throw UnimplementedError("52");
  }

  Future<void> sessionCloseVoiceCall(
      {required UuidValue sessionId, dynamic hint}) {
    throw UnimplementedError("53");
  }

  Future<void> cmHandleIncomingVoiceCall(
      {required int id, required bool accept, dynamic hint}) {
    throw UnimplementedError("54");
  }

  Future<void> cmCloseVoiceCall({required int id, dynamic hint}) {
    throw UnimplementedError("55");
  }

  Future<String> mainGetLastRemoteId({dynamic hint}) {
    return Future(
        () => js.context.callMethod('getByName', ['option', 'last_remote_id']));
  }

  Future<void> mainGetSoftwareUpdateUrl({dynamic hint}) {
    throw UnimplementedError("56");
  }

  Future<String> mainGetHomeDir({dynamic hint}) {
    return Future.value('');
  }

  Future<String> mainGetLangs({dynamic hint}) {
    return Future(() => js.context.callMethod('getByName', ['langs']));
  }

  Future<String> mainGetTemporaryPassword({dynamic hint}) {
    return Future.value('');
  }

  Future<String> mainGetPermanentPassword({dynamic hint}) {
    return Future.value('');
  }

  Future<String> mainGetFingerprint({dynamic hint}) {
    return Future.value('');
  }

  Future<String> cmGetClientsState({dynamic hint}) {
    throw UnimplementedError("57");
  }

  Future<String?> cmCheckClientsLength({required int length, dynamic hint}) {
    throw UnimplementedError("58");
  }

  Future<int> cmGetClientsLength({dynamic hint}) {
    throw UnimplementedError("59");
  }

  Future<void> mainInit({required String appDir, dynamic hint}) {
    return Future.value();
  }

  Future<void> mainDeviceId({required String id, dynamic hint}) {
    // TODO: ?
    throw UnimplementedError("60");
  }

  Future<void> mainDeviceName({required String name, dynamic hint}) {
    // TODO: ?
    throw UnimplementedError("61");
  }

  Future<void> mainRemovePeer({required String id, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', ['remove', id]));
  }

  bool mainHasHwcodec({dynamic hint}) {
    throw UnimplementedError("62");
  }

  bool mainHasVram({dynamic hint}) {
    throw UnimplementedError("63");
  }

  String mainSupportedHwdecodings({dynamic hint}) {
    return '{}';
  }

  Future<bool> mainIsRoot({dynamic hint}) {
    throw UnimplementedError("64");
  }

  int getDoubleClickTime({dynamic hint}) {
    return 500;
  }

  Future<void> mainStartDbusServer({dynamic hint}) {
    throw UnimplementedError("66");
  }

  Future<void> mainSaveAb({required String json, dynamic hint}) {
    throw UnimplementedError("67");
  }

  Future<void> mainClearAb({dynamic hint}) {
    throw UnimplementedError("68");
  }

  Future<String> mainLoadAb({dynamic hint}) {
    throw UnimplementedError("69");
  }

  Future<void> mainSaveGroup({required String json, dynamic hint}) {
    throw UnimplementedError("70");
  }

  Future<void> mainClearGroup({dynamic hint}) {
    throw UnimplementedError("71");
  }

  Future<String> mainLoadGroup({dynamic hint}) {
    throw UnimplementedError("72");
  }

  Future<void> sessionSendPointer(
      {required UuidValue sessionId, required String msg, dynamic hint}) {
    throw UnimplementedError("73");
  }

  Future<void> sessionSendMouse(
      {required UuidValue sessionId, required String msg, dynamic hint}) {
    return Future(
        () => js.context.callMethod('setByName', ['send_mouse', msg]));
  }

  Future<void> sessionRestartRemoteDevice(
      {required UuidValue sessionId, dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', ['restart']));
  }

  String sessionGetAuditServerSync(
      {required UuidValue sessionId, required String typ, dynamic hint}) {
    return js.context.callMethod('getByName', ['audit_server', typ]);
  }

  Future<void> sessionSendNote(
      {required UuidValue sessionId, required String note, dynamic hint}) {
    throw UnimplementedError("74");
  }

  Future<String> sessionAlternativeCodecs(
      {required UuidValue sessionId, dynamic hint}) {
    return Future(
        () => js.context.callMethod('getByName', ['alternative_codecs']));
  }

  Future<void> sessionChangePreferCodec(
      {required UuidValue sessionId, dynamic hint}) {
    return Future(
        () => js.context.callMethod('setByName', ['change_prefer_codec']));
  }

  Future<void> sessionOnWaitingForImageDialogShow(
      {required UuidValue sessionId, dynamic hint}) {
    return Future.value();
  }

  Future<void> sessionToggleVirtualDisplay(
      {required UuidValue sessionId,
      required int index,
      required bool on,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'toggle_virtual_display',
          jsonEncode({'index': index, 'on': on})
        ]));
  }

  Future<void> mainSetHomeDir({required String home, dynamic hint}) {
    throw UnimplementedError("75");
  }

  String mainGetDataDirIos({dynamic hint}) {
    throw UnimplementedError("76");
  }

  Future<void> mainStopService({dynamic hint}) {
    throw UnimplementedError("77");
  }

  Future<void> mainStartService({dynamic hint}) {
    throw UnimplementedError("78");
  }

  Future<void> mainUpdateTemporaryPassword({dynamic hint}) {
    throw UnimplementedError("79");
  }

  Future<void> mainSetPermanentPassword(
      {required String password, dynamic hint}) {
    throw UnimplementedError("80");
  }

  Future<bool> mainCheckSuperUserPermission({dynamic hint}) {
    throw UnimplementedError("81");
  }

  Future<void> mainCheckMouseTime({dynamic hint}) {
    throw UnimplementedError("82");
  }

  Future<double> mainGetMouseTime({dynamic hint}) {
    throw UnimplementedError("83");
  }

  Future<void> mainWol({required String id, dynamic hint}) {
    throw UnimplementedError("84");
  }

  Future<void> mainCreateShortcut({required String id, dynamic hint}) {
    // TODO:
    throw UnimplementedError("85");
  }

  Future<void> cmSendChat(
      {required int connId, required String msg, dynamic hint}) {
    throw UnimplementedError("86");
  }

  Future<void> cmLoginRes(
      {required int connId, required bool res, dynamic hint}) {
    throw UnimplementedError("87");
  }

  Future<void> cmCloseConnection({required int connId, dynamic hint}) {
    throw UnimplementedError("88");
  }

  Future<void> cmRemoveDisconnectedConnection(
      {required int connId, dynamic hint}) {
    throw UnimplementedError("89");
  }

  Future<void> cmCheckClickTime({required int connId, dynamic hint}) {
    throw UnimplementedError("90");
  }

  Future<double> cmGetClickTime({dynamic hint}) {
    throw UnimplementedError("91");
  }

  Future<void> cmSwitchPermission(
      {required int connId,
      required String name,
      required bool enabled,
      dynamic hint}) {
    throw UnimplementedError("92");
  }

  bool cmCanElevate({dynamic hint}) {
    throw UnimplementedError("93");
  }

  Future<void> cmElevatePortable({required int connId, dynamic hint}) {
    throw UnimplementedError("94");
  }

  Future<void> cmSwitchBack({required int connId, dynamic hint}) {
    throw UnimplementedError("95");
  }

  Future<String> cmGetConfig({required String name, dynamic hint}) {
    throw UnimplementedError("96");
  }

  Future<String> mainGetBuildDate({dynamic hint}) {
    return Future(() => js.context.callMethod('getByName', ['build_date']));
  }

  String translate(
      {required String name, required String locale, dynamic hint}) {
    return js.context.callMethod('getByName', [
      'translate',
      jsonEncode({'locale': locale, 'text': name})
    ]);
  }

  int sessionGetRgbaSize(
      {required UuidValue sessionId, required int display, dynamic hint}) {
    return 0;
  }

  void sessionNextRgba(
      {required UuidValue sessionId, required int display, dynamic hint}) {}

  void sessionRegisterPixelbufferTexture(
      {required UuidValue sessionId,
      required int display,
      required int ptr,
      dynamic hint}) {}

  void sessionRegisterGpuTexture(
      {required UuidValue sessionId,
      required int display,
      required int ptr,
      dynamic hint}) {}

  Future<void> queryOnlines({required List<String> ids, dynamic hint}) {
    return Future(() =>
        js.context.callMethod('setByName', ['query_onlines', jsonEncode(ids)]));
  }

  // Dup to the function in hbb_common, lib.rs
  // Maybe we need to move this function to js part.
  int versionToNumber({required String v, dynamic hint}) {
    return int.tryParse(
            js.context.callMethod('getByName', ['get_version_number', v])) ??
        0;
  }

  Future<bool> optionSynced({dynamic hint}) {
    return Future.value(true);
  }

  bool mainIsInstalled({dynamic hint}) {
    throw UnimplementedError("97");
  }

  void mainInitInputSource({dynamic hint}) {
    throw UnimplementedError("98");
  }

  bool mainIsInstalledLowerVersion({dynamic hint}) {
    throw UnimplementedError("99");
  }

  bool mainIsInstalledDaemon({required bool prompt, dynamic hint}) {
    throw UnimplementedError("100");
  }

  bool mainIsProcessTrusted({required bool prompt, dynamic hint}) {
    throw UnimplementedError("101");
  }

  bool mainIsCanScreenRecording({required bool prompt, dynamic hint}) {
    throw UnimplementedError("102");
  }

  bool mainIsCanInputMonitoring({required bool prompt, dynamic hint}) {
    throw UnimplementedError("103");
  }

  bool mainIsShareRdp({dynamic hint}) {
    throw UnimplementedError("104");
  }

  Future<void> mainSetShareRdp({required bool enable, dynamic hint}) {
    throw UnimplementedError("105");
  }

  bool mainGotoInstall({dynamic hint}) {
    throw UnimplementedError("106");
  }

  String mainGetNewVersion({dynamic hint}) {
    throw UnimplementedError("107");
  }

  bool mainUpdateMe({dynamic hint}) {
    throw UnimplementedError("108");
  }

  Future<void> setCurSessionId({required UuidValue sessionId, dynamic hint}) {
    throw UnimplementedError("109");
  }

  bool installShowRunWithoutInstall({dynamic hint}) {
    throw UnimplementedError("110");
  }

  Future<void> installRunWithoutInstall({dynamic hint}) {
    throw UnimplementedError("111");
  }

  Future<void> installInstallMe(
      {required String options, required String path, dynamic hint}) {
    throw UnimplementedError("112");
  }

  String installInstallPath({dynamic hint}) {
    throw UnimplementedError("113");
  }

  Future<void> mainAccountAuth(
      {required String op, required bool rememberMe, dynamic hint}) {
    throw UnimplementedError("114");
  }

  Future<void> mainAccountAuthCancel({dynamic hint}) {
    throw UnimplementedError("115");
  }

  Future<String> mainAccountAuthResult({dynamic hint}) {
    throw UnimplementedError("116");
  }

  Future<void> mainOnMainWindowClose({dynamic hint}) {
    throw UnimplementedError("117");
  }

  bool mainCurrentIsWayland({dynamic hint}) {
    return false;
  }

  bool mainIsLoginWayland({dynamic hint}) {
    return false;
  }

  bool mainHideDock({dynamic hint}) {
    throw UnimplementedError("118");
  }

  bool mainHasFileClipboard({dynamic hint}) {
    return false;
  }

  bool mainHasGpuTextureRender({dynamic hint}) {
    return false;
  }

  Future<void> cmInit({dynamic hint}) {
    throw UnimplementedError("119");
  }

  Future<void> mainStartIpcUrlServer({dynamic hint}) {
    throw UnimplementedError("120");
  }

  Future<void> mainTestWallpaper({required int second, dynamic hint}) {
    // TODO: implement mainTestWallpaper
    return Future.value();
  }

  Future<bool> mainSupportRemoveWallpaper({dynamic hint}) {
    // TODO: implement mainSupportRemoveWallpaper
    return Future.value(false);
  }

  bool isIncomingOnly({dynamic hint}) {
    return false;
  }

  bool isOutgoingOnly({dynamic hint}) {
    return false;
  }

  bool isCustomClient({dynamic hint}) {
    return false;
  }

  bool isDisableSettings({dynamic hint}) {
    return false;
  }

  bool isDisableAb({dynamic hint}) {
    return false;
  }

  bool isDisableGroupPanel({dynamic hint}) {
    return false;
  }

  bool isDisableAccount({dynamic hint}) {
    return false;
  }

  bool isDisableInstallation({dynamic hint}) {
    return false;
  }

  Future<bool> isPresetPassword({dynamic hint}) {
    return Future.value(false);
  }

  Future<void> sendUrlScheme({required String url, dynamic hint}) {
    throw UnimplementedError("121");
  }

  Future<void> pluginEvent(
      {required String id,
      required String peer,
      required Uint8List event,
      dynamic hint}) {
    throw UnimplementedError("122");
  }

  Stream<EventToUI> pluginRegisterEventStream(
      {required String id, dynamic hint}) {
    throw UnimplementedError("123");
  }

  String? pluginGetSessionOption(
      {required String id,
      required String peer,
      required String key,
      dynamic hint}) {
    throw UnimplementedError("124");
  }

  Future<void> pluginSetSessionOption(
      {required String id,
      required String peer,
      required String key,
      required String value,
      dynamic hint}) {
    throw UnimplementedError("125");
  }

  String? pluginGetSharedOption(
      {required String id, required String key, dynamic hint}) {
    throw UnimplementedError("126");
  }

  Future<void> pluginSetSharedOption(
      {required String id,
      required String key,
      required String value,
      dynamic hint}) {
    throw UnimplementedError("127");
  }

  Future<void> pluginReload({required String id, dynamic hint}) {
    throw UnimplementedError("128");
  }

  void pluginEnable({required String id, required bool v, dynamic hint}) {
    throw UnimplementedError("129");
  }

  bool pluginIsEnabled({required String id, dynamic hint}) {
    throw UnimplementedError("130");
  }

  bool pluginFeatureIsEnabled({dynamic hint}) {
    throw UnimplementedError("131");
  }

  Future<void> pluginSyncUi({required String syncTo, dynamic hint}) {
    throw UnimplementedError("132");
  }

  Future<void> pluginListReload({dynamic hint}) {
    throw UnimplementedError("133");
  }

  Future<void> pluginInstall(
      {required String id, required bool b, dynamic hint}) {
    throw UnimplementedError("134");
  }

  bool isSupportMultiUiSession({required String version, dynamic hint}) {
    return versionToNumber(v: version) > versionToNumber(v: '1.2.4');
  }

  bool isSelinuxEnforcing({dynamic hint}) {
    return false;
  }

  String mainDefaultPrivacyModeImpl({dynamic hint}) {
    throw UnimplementedError("135");
  }

  String mainSupportedPrivacyModeImpls({dynamic hint}) {
    throw UnimplementedError("136");
  }

  String mainSupportedInputSource({dynamic hint}) {
    return jsonEncode([
      ['Input source 1', 'input_source_1_tip'],
      ['Input source 2', 'input_source_2_tip']
    ]);
  }

  Future<String> mainGenerate2Fa({dynamic hint}) {
    throw UnimplementedError("137");
  }

  Future<bool> mainVerify2Fa({required String code, dynamic hint}) {
    throw UnimplementedError("138");
  }

  bool mainHasValid2FaSync({dynamic hint}) {
    throw UnimplementedError("139");
  }

  String mainGetHardOption({required String key, dynamic hint}) {
    throw UnimplementedError("140");
  }

  Future<void> mainCheckHwcodec({dynamic hint}) {
    throw UnimplementedError("141");
  }

  Future<void> sessionRequestNewDisplayInitMsgs(
      {required UuidValue sessionId, required int display, dynamic hint}) {
    throw UnimplementedError("142");
  }

  Future<String> mainHandleWaylandScreencastRestoreToken(
      {required String key, required String value, dynamic hint}) {
    throw UnimplementedError("143");
  }

  bool mainIsOptionFixed({required String key, dynamic hint}) {
    return false;
  }

  bool mainGetUseTextureRender({dynamic hint}) {
    throw UnimplementedError("144");
  }

  bool mainHasValidBotSync({dynamic hint}) {
    throw UnimplementedError("145");
  }

  Future<String> mainVerifyBot({required String token, dynamic hint}) {
    throw UnimplementedError("146");
  }

  String mainGetUnlockPin({dynamic hint}) {
    throw UnimplementedError("147");
  }

  String mainSetUnlockPin({required String pin, dynamic hint}) {
    throw UnimplementedError("148");
  }

  bool sessionGetEnableTrustedDevices(
      {required UuidValue sessionId, dynamic hint}) {
    throw UnimplementedError("149");
  }

  Future<String> mainGetTrustedDevices({dynamic hint}) {
    throw UnimplementedError("150");
  }

  Future<void> mainRemoveTrustedDevices({required String json, dynamic hint}) {
    throw UnimplementedError("151");
  }

  Future<void> mainClearTrustedDevices({dynamic hint}) {
    throw UnimplementedError("152");
  }

  Future<String> getVoiceCallInputDevice({required bool isCm, dynamic hint}) {
    throw UnimplementedError("153");
  }

  Future<void> setVoiceCallInputDevice(
      {required bool isCm, required String device, dynamic hint}) {
    throw UnimplementedError("154");
  }

  bool isPresetPasswordMobileOnly({dynamic hint}) {
    throw UnimplementedError("155");
  }

  String mainGetBuildinOption({required String key, dynamic hint}) {
    return '';
  }

  String installInstallOptions({dynamic hint}) {
    throw UnimplementedError("156");
  }

  int mainMaxEncryptLen({dynamic hint}) {
    throw UnimplementedError("157");
  }

  sessionRenameFile(
      {required UuidValue sessionId,
      required int actId,
      required String path,
      required String newName,
      required bool isRemote,
      dynamic hint}) {
    return Future(() => js.context.callMethod('setByName', [
          'rename_file',
          jsonEncode({
            'id': actId,
            'path': path,
            'new_name': newName,
            'is_remote': isRemote
          })
        ]));
  }

  void dispose() {}
}
