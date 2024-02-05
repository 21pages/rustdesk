import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_hbb/common/hbbs/hbbs.dart';
import 'package:flutter_hbb/common/widgets/peers_view.dart';
import 'package:flutter_hbb/models/model.dart';
import 'package:flutter_hbb/models/peer_model.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:get/get.dart';
import 'package:bot_toast/bot_toast.dart';
import 'package:http/http.dart' as http;

import '../common.dart';

// TODO: pull after change

final syncAbOption = 'sync-ab-with-recent-sessions';
bool shouldSyncAb() {
  return bind.mainGetLocalOption(key: syncAbOption).isNotEmpty;
}

final sortAbTagsOption = 'sync-ab-tags';
bool shouldSortTags() {
  return bind.mainGetLocalOption(key: sortAbTagsOption).isNotEmpty;
}

final filterAbTagOption = 'filter-ab-by-intersection';
bool filterAbTagByIntersection() {
  return bind.mainGetLocalOption(key: filterAbTagOption).isNotEmpty;
}

const personalAddressBookName = "Personal";

class AbModel {
  final addressbooks = Map<String, BaseAb>.fromEntries([]).obs;
  final personalAddressBook = PersonalAb();
  List<SharedAbProfile> sharedAbProfiles = List.empty(growable: true);
  final RxString _currentName = personalAddressBookName.obs;
  RxString get currentName => _currentName;
  BaseAb get current => addressbooks[_currentName.value] ?? personalAddressBook;

  RxList<Peer> get currentAbPeers => current.peers;
  RxList get currentAbTags => current.tags;
  RxList get selectedTags => current.selectedTags;

  RxBool get currentAbLoading => current.abLoading;
  RxString get currentAbPullError => current.pullError;
  RxString get currentAbPushError => current.pushError;
  bool get currentAbEmtpy => currentAbPeers.isEmpty && currentAbTags.isEmpty;
  RxBool supportSharedAb = false.obs;

  final sortTags = shouldSortTags().obs;
  final filterByIntersection = filterAbTagByIntersection().obs;

  // licensedDevices is obtained from personal ab, shared ab restrict it in server
  var licensedDevices = 0;

  var _syncAllFromRecent = true;
  var _syncFromRecentLock = false;
  var _resetting = false;
  var _timerCounter = 0;
  var _cacheLoadOnceFlag = false;
  var _everPulled = false;

  WeakReference<FFI> parent;

  AbModel(this.parent) {
    addressbooks[personalAddressBookName] = personalAddressBook;
    if (desktopType == DesktopType.main) {
      Timer.periodic(Duration(milliseconds: 500), (timer) async {
        if (_timerCounter++ % 6 == 0) {
          if (!gFFI.userModel.isLogin) return;
          if (addressbooks.values.any((e) => !e.initialized)) return;
          if (_resetting) return;
          syncFromRecent();
        }
      });
    }
  }

  reset() async {
    print("reset ab model");
    _resetting = true;
    sharedAbProfiles.clear();
    addressbooks.removeWhere((key, _) => key != personalAddressBookName);
    setCurrentName(personalAddressBookName);
    personalAddressBook.reset();
    await bind.mainClearAb();
    licensedDevices = 0;
    _resetting = false;
  }

// #region ab
  Future<void> pullAb({force = true, quiet = false}) async {
    await _pullAb(force: force, quiet: quiet);
    platformFFI.tryHandle({'name': LoadEvent.addressBook});
  }

  Future<void> _pullAb({force = true, quiet = false}) async {
    debugPrint("pullAb, force:$force, quiet:$quiet");
    if (!gFFI.userModel.isLogin) return;
    try {
      // get all address book name
      List<SharedAbProfile> tmpSharedAbs = List.empty(growable: true);
      await _getSharedAbProfiles(tmpSharedAbs);
      sharedAbProfiles = tmpSharedAbs;
      addressbooks.removeWhere((key, _) => key != personalAddressBookName);
      for (int i = 0; i < sharedAbProfiles.length; i++) {
        SharedAbProfile p = sharedAbProfiles[i];
        addressbooks[p.name] = SharedAb(p);
      }
      // set current address book name
      if (!_everPulled) {
        _everPulled = true;
        final name = bind.getLocalFlutterOption(k: 'current-ab-name');
        if (addressbooks.containsKey(name)) {
          _currentName.value = name;
        }
      }
      if (!addressbooks.containsKey(_currentName.value)) {
        setCurrentName(personalAddressBookName);
      }
      // pull shared ab data, current first
      await current.pullAb(force: force, quiet: quiet);
      addressbooks.forEach((key, value) async {
        if (key != _currentName.value) {
          return await value.pullAb(force: force, quiet: quiet);
        }
      });
      _saveCache();
    } catch (e) {
      debugPrint("pullAb error: $e");
    }
    // again in case of error happens
    if (!addressbooks.containsKey(_currentName.value)) {
      setCurrentName(personalAddressBookName);
    }
    _syncAllFromRecent = true;
  }

  Future<bool> _getSharedAbProfiles(List<SharedAbProfile> tmpSharedAbs) async {
    final api = "${await bind.mainGetApiServer()}/api/ab/shared/profiles";
    try {
      var uri0 = Uri.parse(api);
      final pageSize = 100;
      var total = 0;
      int current = 0;
      do {
        current += 1;
        var uri = Uri(
            scheme: uri0.scheme,
            host: uri0.host,
            path: uri0.path,
            port: uri0.port,
            queryParameters: {
              'current': current.toString(),
              'pageSize': pageSize.toString(),
            });
        var headers = getHttpHeaders();
        headers['Content-Type'] = "application/json";
        final resp = await http.post(uri, headers: headers);
        supportSharedAb.value = resp.statusCode != 404;
        if (resp.statusCode == 404) {
          debugPrint(
              "HTTP 404, api server doesn't support shared address book");
          return false;
        }
        Map<String, dynamic> json =
            _jsonDecodeResp(utf8.decode(resp.bodyBytes), resp.statusCode);
        if (json.containsKey('error')) {
          throw json['error'];
        }
        if (resp.statusCode != 200) {
          throw 'HTTP ${resp.statusCode}';
        }
        if (json.containsKey('total')) {
          if (total == 0) total = json['total'];
          if (json.containsKey('data')) {
            final data = json['data'];
            if (data is List) {
              for (final profile in data) {
                final u = SharedAbProfile.fromJson(profile);
                int index = tmpSharedAbs.indexWhere((e) => e.name == u.name);
                if (index < 0) {
                  tmpSharedAbs.add(u);
                } else {
                  tmpSharedAbs[index] = u;
                }
              }
            }
          }
        }
      } while (current * pageSize < total);
      return true;
    } catch (err) {
      debugPrint('_getSharedAbProfiles err: ${err.toString()}');
    }
    return false;
  }

  Future<String> addSharedAb(
      String name, String note, bool edit, bool pwd) async {
    if (addressbooks.containsKey(name)) {
      return '$name already exists';
    }
    final api = "${await bind.mainGetApiServer()}/api/ab/shared/add";
    var headers = getHttpHeaders();
    headers['Content-Type'] = "application/json";
    var v = {
      'name': name,
      'edit': edit,
      'pwd': pwd,
    };
    if (note.isNotEmpty) {
      v['note'] = note;
    }
    final body = jsonEncode(v);
    final resp = await http.post(Uri.parse(api), headers: headers, body: body);
    final errMsg = _jsonDecodeActionResp(resp);
    return errMsg;
  }

  Future<String> updateSharedAb(
      String guid, String name, String note, bool edit, bool pwd) async {
    final api = "${await bind.mainGetApiServer()}/api/ab/shared/update/profile";
    var headers = getHttpHeaders();
    headers['Content-Type'] = "application/json";
    var v = {
      'guid': guid,
      'name': name,
      'edit': edit,
      'pwd': pwd,
    };
    if (note.isNotEmpty) {
      v['note'] = note;
    }
    final body = jsonEncode(v);
    final resp = await http.put(Uri.parse(api), headers: headers, body: body);
    final errMsg = _jsonDecodeActionResp(resp);
    return errMsg;
  }

  Future<String> deleteSharedAb(String name) async {
    final guid = sharedAbProfiles.firstWhereOrNull((e) => e.name == name)?.guid;
    if (guid == null) {
      return '$name not found';
    }
    final api = "${await bind.mainGetApiServer()}/api/ab/shared";
    var headers = getHttpHeaders();
    headers['Content-Type'] = "application/json";
    final body = jsonEncode([guid]);
    final resp =
        await http.delete(Uri.parse(api), headers: headers, body: body);
    final errMsg = _jsonDecodeActionResp(resp);
    return errMsg;
  }

// #endregion

// #region permission
  bool allowedToEditCurrentAb() {
    if (_currentName.value == personalAddressBookName) {
      return true;
    }
    SharedAbProfile? profile = current.sharedProfile();
    if (profile == null) {
      return false;
    }
    final creatorOrAdmin = profile.creator == gFFI.userModel.userName.value ||
        gFFI.userModel.isAdmin.value;
    return creatorOrAdmin || (profile.edit ?? false);
  }

  bool allowedToUpdateCurrentProfile() {
    if (_currentName.value == personalAddressBookName) {
      return false;
    }
    SharedAbProfile? profile = current.sharedProfile();
    if (profile == null) {
      return false;
    }
    final creatorOrAdmin = profile.creator == gFFI.userModel.userName.value ||
        gFFI.userModel.isAdmin.value;
    return creatorOrAdmin;
  }

  bool isCurrentAbSharingPassword() {
    if (_currentName.value == personalAddressBookName) {
      return false;
    }
    SharedAbProfile? profile = current.sharedProfile();
    if (profile == null) {
      return false;
    }
    return profile.pwd ?? false;
  }

  List<String> addressBooksAllowedToDelete() {
    List<String> list = [];
    addressbooks.forEach((key, value) async {
      if (key == personalAddressBookName) return;
      final profile = value.sharedProfile();
      if (profile == null) return;
      if (profile.creator == gFFI.userModel.userName.value ||
          gFFI.userModel.isAdmin.value) {
        list.add(key);
      }
    });
    return list;
  }
// #endregion

// #region Peer
  Future<bool> addIdToCurrent(
      String id, String alias, String password, List<dynamic> tags) async {
    if (currentAbPeers.where((element) => element.id == id).isNotEmpty) {
      return false;
    }
    final peer = Peer.fromJson({
      'id': id,
      'alias': alias,
      'tags': tags,
    });
    if (password.isNotEmpty) {
      peer.password = password;
    }
    _mergePeerFromGroup(peer);
    final ret = await addPeersTo([peer], _currentName.value);
    if (_currentName.value != personalAddressBookName) {
      await current.pullAb(force: true, quiet: true);
    }
    return ret;
  }

  Future<bool> addPeersTo(List<Peer> ps, String name) async {
    final ab = addressbooks[name];
    if (ab == null) {
      debugPrint('addPeersTo, no such addressbook: $name');
      return false;
    }
    bool res = await ab.addPeers(ps);
    if (name == _currentName.value) {
      currentAbPeers.refresh();
      platformFFI.tryHandle({'name': LoadEvent.addressBook});
    }
    _syncAllFromRecent = true;
    _saveCache();
    return res;
  }

  Future<bool> changeTagForPeers(List<String> ids, List<dynamic> tags) async {
    bool ret = await current.changeTagForPeers(ids, tags);
    currentAbPeers.refresh();
    _saveCache();
    return ret;
  }

  Future<bool> changeAlias({required String id, required String alias}) async {
    bool res = await current.changeAlias(id: id, alias: alias);
    currentAbPeers.refresh();
    _saveCache();
    return res;
  }

  Future<bool> changePersonalHashPassword(
      String id, String hash, bool toast) async {
    final ret =
        await personalAddressBook.changePersonalHashPassword(id, hash, toast);
    _saveCache();
    return ret;
  }

  Future<bool> changeSharedPassword(
      String abName, String id, String password) async {
    return await addressbooks[abName]?.changeSharedPassword(id, password) ??
        false;
  }

  Future<bool> deletePeers(List<String> ids) async {
    final ret = await current.deletePeers(ids);
    currentAbPeers.refresh();
    platformFFI.tryHandle({'name': LoadEvent.addressBook});
    _saveCache();
    if (_currentName.value == personalAddressBookName) {
      Future.delayed(Duration(seconds: 2), () async {
        if (!shouldSyncAb()) return;
        var hasSynced = false;
        for (var id in ids) {
          if (await bind.mainPeerExists(id: id)) {
            hasSynced = true;
            break;
          }
        }
        if (hasSynced) {
          BotToast.showText(
              contentColor: Colors.lightBlue,
              text: translate('synced_peer_readded_tip'));
          _syncAllFromRecent = true;
        }
      });
    }
    return ret;
  }

// #endregion

// #region Tags
  Future<bool> addTags(List<String> tagList) async {
    final ret = await current.addTags(tagList);
    _saveCache();
    return ret;
  }

  Future<bool> renameTag(String oldTag, String newTag) async {
    final ret = await current.renameTag(oldTag, newTag);
    _saveCache();
    return ret;
  }

  Future<bool> setTagColor(String tag, Color color) async {
    final ret = await current.setTagColor(tag, color);
    _saveCache();
    return ret;
  }

  Future<bool> deleteTag(String tag) async {
    final ret = await current.deleteTag(tag);
    _saveCache();
    return ret;
  }

// #endregion

  Future<void> syncFromRecent({bool push = true}) async {
    if (!_syncFromRecentLock) {
      _syncFromRecentLock = true;
      await _syncFromRecentWithoutLock(push: push);
      _syncFromRecentLock = false;
    }
  }

  Future<void> _syncFromRecentWithoutLock({bool push = true}) async {
    Future<List<Peer>> getRecentPeers() async {
      try {
        List<String> filteredPeerIDs;
        if (_syncAllFromRecent) {
          _syncAllFromRecent = false;
          filteredPeerIDs = [];
        } else {
          final new_stored_str = await bind.mainGetNewStoredPeers();
          if (new_stored_str.isEmpty) return [];
          filteredPeerIDs = (jsonDecode(new_stored_str) as List<dynamic>)
              .map((e) => e.toString())
              .toList();
          if (filteredPeerIDs.isEmpty) return [];
        }
        final loadStr = await bind.mainLoadRecentPeersForAb(
            filter: jsonEncode(filteredPeerIDs));
        if (loadStr.isEmpty) {
          return [];
        }
        List<dynamic> mapPeers = jsonDecode(loadStr);
        List<Peer> recents = List.empty(growable: true);
        for (var m in mapPeers) {
          if (m is Map<String, dynamic>) {
            recents.add(Peer.fromJson(m));
          }
        }
        return recents;
      } catch (e) {
        debugPrint('getRecentPeers: $e');
      }
      return [];
    }

    try {
      if (!shouldSyncAb()) return;
      final recents = await getRecentPeers();
      if (recents.isEmpty) return;
      addressbooks.forEach((key, value) async {
        await value.syncFromRecent(recents, true);
      });
    } catch (e) {
      debugPrint('_syncFromRecentWithoutLock: $e');
    }
  }

  _saveCache() {
    // TODO: too many _saveCache
    try {
      var ab_entries = _serializeCache();
      Map<String, dynamic> m = <String, dynamic>{
        "access_token": bind.mainGetLocalOption(key: 'access_token'),
        "ab_entries": ab_entries,
      };
      bind.mainSaveAb(json: jsonEncode(m));
    } catch (e) {
      debugPrint('ab save:$e');
    }
  }

  List<dynamic> _serializeCache() {
    var res = [];
    addressbooks.forEach((key, value) {
      res.add({
        "guid": value.sharedProfile()?.guid ?? '',
        "name": key,
        "tags": value.tags,
        "peers": value.peers
            .map((e) => key == personalAddressBookName
                ? e.toPersonalAbUploadJson()
                : e.toSharedAbCacheJson())
            .toList(),
        "tag_colors": jsonEncode(value.tagColors)
      });
    });
    return res;
  }

  Future<void> loadCache() async {
    try {
      if (_cacheLoadOnceFlag || currentAbLoading.value) return;
      _cacheLoadOnceFlag = true;
      final access_token = bind.mainGetLocalOption(key: 'access_token');
      if (access_token.isEmpty) return;
      final cache = await bind.mainLoadAb();
      if (currentAbLoading.value) return;
      final data = jsonDecode(cache);
      if (data == null || data['access_token'] != access_token) return;
      _deserializeCache(data);
    } catch (e) {
      debugPrint("load ab cache: $e");
    }
  }

  _deserializeCache(dynamic data) {
    if (data == null) return;
    reset();
    final abEntries = data['ab_entries'];
    if (abEntries is List) {
      for (var i = 0; i < abEntries.length; i++) {
        var abEntry = abEntries[i];
        if (abEntry is Map<String, dynamic>) {
          var guid = abEntry['guid'];
          var name = abEntry['name'];
          final BaseAb ab;
          if (name == personalAddressBookName) {
            ab = personalAddressBook;
          } else {
            if (name == null || guid == null) {
              continue;
            }
            ab = SharedAb(SharedAbProfile(guid, name, '', '', false, false));
            addressbooks[name] = ab;
          }
          if (abEntry['tags'] is List) {
            ab.tags.value = abEntry['tags'];
          }
          if (abEntry['peers'] is List) {
            for (var peer in abEntry['peers']) {
              ab.peers.add(Peer.fromJson(peer));
            }
          }
          if (abEntry['tag_colors'] is String) {
            Map<String, dynamic> map = jsonDecode(abEntry['tag_colors']);
            ab.tagColors.value = Map<String, int>.from(map);
          }
        }
      }
    }
  }

// #region Tools
  Peer? find(String id) {
    return currentAbPeers.firstWhereOrNull((e) => e.id == id);
  }

  bool idContainByCurrent(String id) {
    return currentAbPeers.where((element) => element.id == id).isNotEmpty;
  }

  void unsetSelectedTags() {
    selectedTags.clear();
  }

  List<dynamic> getPeerTags(String id) {
    final it = currentAbPeers.where((p0) => p0.id == id);
    if (it.isEmpty) {
      return [];
    } else {
      return it.first.tags;
    }
  }

  Color getCurrentAbTagColor(String tag) {
    int? colorValue = current.tagColors[tag];
    if (colorValue != null) {
      return Color(colorValue);
    }
    return str2color2(tag, existing: current.tagColors.values.toList());
  }

  _mergePeerFromGroup(Peer p) {
    final g = gFFI.groupModel.peers.firstWhereOrNull((e) => p.id == e.id);
    if (g == null) return;
    if (p.username.isEmpty) {
      p.username = g.username;
    }
    if (p.hostname.isEmpty) {
      p.hostname = g.hostname;
    }
    if (p.platform.isEmpty) {
      p.platform = g.platform;
    }
  }

  List<String> addressBookNames() {
    return addressbooks.keys.toList();
  }

  void setCurrentName(String name) {
    if (addressbooks.containsKey(name)) {
      _currentName.value = name;
    } else {
      _currentName.value = personalAddressBookName;
    }
    platformFFI.tryHandle({'name': LoadEvent.addressBook});
  }

  bool isCurrentAbFull(bool warn) {
    return current.isFull(warn);
  }
// #endregion
}

abstract class BaseAb {
  final peers = List<Peer>.empty(growable: true).obs;
  final tags = [].obs;
  final RxMap<String, int> tagColors = Map<String, int>.fromEntries([]).obs;
  final selectedTags = List<String>.empty(growable: true).obs;

  final pullError = "".obs;
  final pushError = "".obs;
  final abLoading = false.obs;
  var initialized = false;

  reset() {
    pullError.value = '';
    pushError.value = '';
    tags.clear();
    peers.clear();
    initialized = false;
  }

  Future<void> pullAb({force = true, quiet = false}) async {
    if (abLoading.value) return;
    if (!force && initialized) return;
    if (!quiet) {
      abLoading.value = true;
      pullError.value = "";
    }
    final ret = pullAbImpl(force: force, quiet: quiet);
    abLoading.value = false;
    initialized = true;
    return ret;
  }

  Future<void> pullAbImpl({force = true, quiet = false});

  Future<bool> addPeers(List<Peer> ps);
  void addPeerWithoutPush(Peer peer) {
    final index = peers.indexWhere((e) => e.id == peer.id);
    if (index >= 0) {
      _merge(peer, peers[index]);
    } else {
      peers.add(peer);
    }
  }

  Future<bool> changeTagForPeers(List<String> ids, List<dynamic> tags);
  void changeTagForPeersWithoutPush(List<String> ids, List<dynamic> tags) {
    peers.map((e) {
      if (ids.contains(e.id)) {
        e.tags = tags;
      }
    }).toList();
  }

  Future<bool> changeAlias({required String id, required String alias});
  void changeAliasWithoutPush({required String id, required String alias}) {
    final it = peers.where((element) => element.id == id);
    if (it.isEmpty) {
      return;
    }
    it.first.alias = alias;
  }

  Future<bool> changeSharedPassword(String id, String password);

  Future<bool> deletePeers(List<String> ids);
  void deletePeersWithoutPush(List<String> ids) {
    peers.removeWhere((e) => ids.contains(e.id));
  }

  Future<bool> addTags(List<String> tagList);
  void addTagsWithoutPush(List<String> tagList) {
    for (var e in tagList) {
      if (!tagContainBy(e)) {
        tags.add(e);
      }
    }
  }

  bool tagContainBy(String tag) {
    return tags.where((element) => element == tag).isNotEmpty;
  }

  Future<bool> renameTag(String oldTag, String newTag);
  void renameTagWithoutPush(String oldTag, String newTag) {
    tags.value = tags.map((e) {
      if (e == oldTag) {
        return newTag;
      } else {
        return e;
      }
    }).toList();
    selectedTags.value = selectedTags.map((e) {
      if (e == oldTag) {
        return newTag;
      } else {
        return e;
      }
    }).toList();
    for (var peer in peers) {
      peer.tags = peer.tags.map((e) {
        if (e == oldTag) {
          return newTag;
        } else {
          return e;
        }
      }).toList();
    }
    int? oldColor = tagColors[oldTag];
    if (oldColor != null) {
      tagColors.remove(oldTag);
      tagColors.addAll({newTag: oldColor});
    }
  }

  Future<bool> setTagColor(String tag, Color color);
  void setTagColorWithoutPush(String tag, Color color) {
    if (tags.contains(tag)) {
      tagColors[tag] = color.value;
    }
  }

  Future<bool> deleteTag(String tag);
  void deleteTagWithoutPush(String tag) {
    gFFI.abModel.selectedTags.remove(tag);
    tags.removeWhere((element) => element == tag);
    tagColors.remove(tag);
    for (var peer in peers) {
      if (peer.tags.isEmpty) {
        continue;
      }
      if (peer.tags.contains(tag)) {
        peer.tags.remove(tag);
      }
    }
  }

  bool isFull(bool warn) {
    final res = gFFI.abModel.licensedDevices > 0 &&
        peers.length >= gFFI.abModel.licensedDevices;
    if (res && warn) {
      BotToast.showText(
          contentColor: Colors.red, text: translate("exceed_max_devices"));
    }
    return res;
  }

  SharedAbProfile? sharedProfile();

  Future<void> syncFromRecent(List<Peer> recents, bool push);
}

class PersonalAb extends BaseAb {
  final sortTags = shouldSortTags().obs;
  final filterByIntersection = filterAbTagByIntersection().obs;
  bool get emtpy => peers.isEmpty && tags.isEmpty;

  PersonalAb();

  @override
  SharedAbProfile? sharedProfile() {
    return null;
  }

  @override
  Future<void> pullAbImpl({force = true, quiet = false}) async {
    final api = "${await bind.mainGetApiServer()}/api/ab";
    int? statusCode;
    try {
      var authHeaders = getHttpHeaders();
      authHeaders['Content-Type'] = "application/json";
      authHeaders['Accept-Encoding'] = "gzip";
      final resp = await http.get(Uri.parse(api), headers: authHeaders);
      statusCode = resp.statusCode;
      if (resp.body.toLowerCase() == "null") {
        // normal reply, emtpy ab return null
        tags.clear();
        tagColors.clear();
        peers.clear();
      } else if (resp.body.isNotEmpty) {
        Map<String, dynamic> json =
            _jsonDecodeResp(utf8.decode(resp.bodyBytes), resp.statusCode);
        if (json.containsKey('error')) {
          throw json['error'];
        } else if (json.containsKey('data')) {
          try {
            gFFI.abModel.licensedDevices = json['licensed_devices'];
            // ignore: empty_catches
          } catch (e) {}
          final data = jsonDecode(json['data']);
          if (data != null) {
            _deserialize(data);
            // _saveCache(); // save on success
          }
        }
      }
    } catch (err) {
      if (!quiet) {
        pullError.value =
            '${translate('pull_ab_failed_tip')}: ${translate(err.toString())}';
      }
    } finally {
      if (pullError.isNotEmpty) {
        if (statusCode == 401) {
          gFFI.userModel.reset(resetOther: true);
        }
      }
    }
  }

  Future<bool> pushAb(
      {bool toastIfFail = true, bool toastIfSucc = true}) async {
    debugPrint("pushAb: toastIfFail:$toastIfFail, toastIfSucc:$toastIfSucc");
    if (!gFFI.userModel.isLogin) return false;
    if (!initialized) return false;
    pushError.value = '';
    bool ret = false;
    try {
      //https: //stackoverflow.com/questions/68249333/flutter-getx-updating-item-in-children-list-is-not-reactive
      peers.refresh();
      final api = "${await bind.mainGetApiServer()}/api/ab";
      var authHeaders = getHttpHeaders();
      authHeaders['Content-Type'] = "application/json";
      final body = jsonEncode({"data": jsonEncode(_serialize())});
      http.Response resp;
      // support compression
      if (gFFI.abModel.licensedDevices > 0 && body.length > 1024) {
        authHeaders['Content-Encoding'] = "gzip";
        resp = await http.post(Uri.parse(api),
            headers: authHeaders, body: GZipCodec().encode(utf8.encode(body)));
      } else {
        resp =
            await http.post(Uri.parse(api), headers: authHeaders, body: body);
      }
      if (resp.statusCode == 200 &&
          (resp.body.isEmpty || resp.body.toLowerCase() == 'null')) {
        ret = true;
        // _saveCache();
      } else {
        Map<String, dynamic> json =
            _jsonDecodeResp(utf8.decode(resp.bodyBytes), resp.statusCode);
        if (json.containsKey('error')) {
          throw json['error'];
        } else if (resp.statusCode == 200) {
          ret = true;
          // _saveCache();
        } else {
          throw 'HTTP ${resp.statusCode}';
        }
      }
    } catch (e) {
      pushError.value =
          '${translate('push_ab_failed_tip')}: ${translate(e.toString())}';
    }

    if (!ret && toastIfFail) {
      BotToast.showText(contentColor: Colors.red, text: pushError.value);
    }
    if (ret && toastIfSucc) {
      showToast(translate('Successful'));
    }
    return ret;
  }

// #region Peer
  @override
  Future<bool> addPeers(List<Peer> ps) async {
    bool res = true;
    for (var p in ps) {
      if (!isFull(true)) {
        addPeerWithoutPush(p);
      } else {
        res = false;
        break;
      }
    }
    return await pushAb() && res;
  }

  @override
  Future<bool> changeTagForPeers(List<String> ids, List<dynamic> tags) async {
    changeTagForPeersWithoutPush(ids, tags);
    return await pushAb();
  }

  @override
  Future<bool> changeAlias({required String id, required String alias}) async {
    changeAliasWithoutPush(id: id, alias: alias);
    return await pushAb();
  }

  @override
  Future<bool> changeSharedPassword(String id, String password) async {
    // no need to implement
    return false;
  }

  @override
  Future<void> syncFromRecent(List<Peer> recents, bool push) async {
    bool peerSyncEqual(Peer a, Peer b) {
      return a.hash == b.hash &&
          a.username == b.username &&
          a.platform == b.platform &&
          a.hostname == b.hostname &&
          a.alias == b.alias;
    }

    bool uiChanged = false;
    bool needSync = false;
    for (var i = 0; i < recents.length; i++) {
      var r = recents[i];
      var index = peers.indexWhere((e) => e.id == r.id);
      if (index < 0) {
        if (!isFull(false)) {
          peers.add(r);
          uiChanged = true;
          needSync = true;
        }
      } else {
        Peer old = Peer.copy(peers[index]);
        _merge(r, peers[index]);
        if (!peerSyncEqual(peers[index], old)) {
          needSync = true;
        }
        if (!old.equal(peers[index])) {
          uiChanged = true;
        }
      }
    }
    if (needSync && push) {
      pushAb(toastIfSucc: false, toastIfFail: false);
    } else if (uiChanged &&
        gFFI.abModel.currentName.value == personalAddressBookName) {
      peers.refresh();
    }
  }

  Future<bool> changePersonalHashPassword(
      String id, String hash, bool toast) async {
    bool changed = false;
    final it = peers.where((element) => element.id == id);
    if (it.isNotEmpty) {
      if (it.first.hash != hash) {
        it.first.hash = hash;
        changed = true;
      }
    }
    if (changed) {
      return await pushAb(toastIfSucc: toast, toastIfFail: toast);
    }
    return true;
  }

  @override
  Future<bool> deletePeers(List<String> ids) async {
    deletePeersWithoutPush(ids);
    return await pushAb();
  }
// #endregion

// #region Tag
  @override
  Future<bool> addTags(List<String> tagList) async {
    addTagsWithoutPush(tagList);
    return await pushAb();
  }

  @override
  Future<bool> renameTag(String oldTag, String newTag) async {
    if (tags.contains(newTag)) {
      BotToast.showText(
          contentColor: Colors.red, text: 'Tag $newTag already exists');
      return false;
    }
    renameTagWithoutPush(oldTag, newTag);
    return await pushAb();
  }

  @override
  Future<bool> setTagColor(String tag, Color color) async {
    setTagColorWithoutPush(tag, color);
    return await pushAb();
  }

  @override
  Future<bool> deleteTag(String tag) async {
    deleteTagWithoutPush(tag);
    return await pushAb();
  }

// #endregion

  Map<String, dynamic> _serialize() {
    final peersJsonData = peers.map((e) => e.toPersonalAbUploadJson()).toList();
    final tagColorJsonData = jsonEncode(tagColors);
    return {
      "tags": tags,
      "peers": peersJsonData,
      "tag_colors": tagColorJsonData
    };
  }

  _deserialize(dynamic data) {
    if (data == null) return;
    final oldOnlineIDs = peers.where((e) => e.online).map((e) => e.id).toList();
    tags.clear();
    tagColors.clear();
    peers.clear();
    if (data['tags'] is List) {
      tags.value = data['tags'];
    }
    if (data['peers'] is List) {
      for (final peer in data['peers']) {
        peers.add(Peer.fromJson(peer));
      }
    }
    if (isFull(false)) {
      peers.removeRange(gFFI.abModel.licensedDevices, peers.length);
    }
    // restore online
    peers
        .where((e) => oldOnlineIDs.contains(e.id))
        .map((e) => e.online = true)
        .toList();
    if (data['tag_colors'] is String) {
      Map<String, dynamic> map = jsonDecode(data['tag_colors']);
      tagColors.value = Map<String, int>.from(map);
    }
    // add color to tag
    final tagsWithoutColor =
        tags.toList().where((e) => !tagColors.containsKey(e)).toList();
    for (var t in tagsWithoutColor) {
      tagColors[t] = str2color2(t, existing: tagColors.values.toList()).value;
    }
  }
}

class SharedAb extends BaseAb {
  late final SharedAbProfile profile;
  final sortTags = shouldSortTags().obs;
  final filterByIntersection = filterAbTagByIntersection().obs;
  bool get emtpy => peers.isEmpty && tags.isEmpty;

  SharedAb(this.profile);

  @override
  SharedAbProfile? sharedProfile() {
    return profile;
  }

  @override
  Future<void> pullAbImpl({force = true, quiet = false}) async {
    List<Peer> tmpPeers = [];
    await _fetchPeers(tmpPeers);
    peers.value = tmpPeers;
    List<SharedAbTag> tmpTags = [];
    await _fetchTags(tmpTags);
    tags.value = tmpTags.map((e) => e.name).toList();
    Map<String, int> tmpTagColors = {};
    for (var t in tmpTags) {
      tmpTagColors[t.name] = t.color;
    }
    tagColors.value = tmpTagColors;
  }

  Future<bool> _fetchPeers(List<Peer> tmpPeers) async {
    final api = "${await bind.mainGetApiServer()}/api/ab/shared/peers";
    try {
      var uri0 = Uri.parse(api);
      final pageSize = 100;
      var total = 0;
      int current = 0;
      do {
        current += 1;
        var uri = Uri(
            scheme: uri0.scheme,
            host: uri0.host,
            path: uri0.path,
            port: uri0.port,
            queryParameters: {
              'current': current.toString(),
              'pageSize': pageSize.toString(),
              'ab': profile.guid,
            });
        var headers = getHttpHeaders();
        headers['Content-Type'] = "application/json";
        final resp = await http.post(uri, headers: headers);
        Map<String, dynamic> json =
            _jsonDecodeResp(utf8.decode(resp.bodyBytes), resp.statusCode);
        if (json.containsKey('error')) {
          throw json['error'];
        }
        if (resp.statusCode != 200) {
          throw 'HTTP ${resp.statusCode}';
        }
        if (json.containsKey('total')) {
          if (total == 0) total = json['total'];
          if (json.containsKey('data')) {
            final data = json['data'];
            if (data is List) {
              for (final profile in data) {
                final u = Peer.fromJson(profile);
                int index = tmpPeers.indexWhere((e) => e.id == u.id);
                if (index < 0) {
                  tmpPeers.add(u);
                } else {
                  tmpPeers[index] = u;
                }
              }
            }
          }
        }
      } while (current * pageSize < total);
      return true;
    } catch (err) {
      debugPrint('_fetchPeers err: ${err.toString()}');
    }
    return false;
  }

  Future<bool> _fetchTags(List<SharedAbTag> tmpTags) async {
    final api =
        "${await bind.mainGetApiServer()}/api/ab/shared/tags/${profile.guid}";
    try {
      var uri0 = Uri.parse(api);
      var uri = Uri(
        scheme: uri0.scheme,
        host: uri0.host,
        path: uri0.path,
        port: uri0.port,
      );
      var headers = getHttpHeaders();
      headers['Content-Type'] = "application/json";
      final resp = await http.post(uri, headers: headers);
      List<dynamic> json =
          _jsonDecodeRespList(utf8.decode(resp.bodyBytes), resp.statusCode);
      if (resp.statusCode != 200) {
        throw 'HTTP ${resp.statusCode}';
      }

      for (final d in json) {
        final t = SharedAbTag.fromJson(d);
        int index = tmpTags.indexWhere((e) => e.name == t.name);
        if (index < 0) {
          tmpTags.add(t);
        } else {
          tmpTags[index] = t;
        }
      }
      return true;
    } catch (err) {
      debugPrint('_fetchTags err: ${err.toString()}');
    }
    return false;
  }

// #region Peers
  @override
  Future<bool> addPeers(List<Peer> ps) async {
    final api =
        "${await bind.mainGetApiServer()}/api/ab/shared/peer/add/${profile.guid}";
    var headers = getHttpHeaders();
    headers['Content-Type'] = "application/json";
    for (var p in ps) {
      if (peers.firstWhereOrNull((e) => e.id == p.id) != null) {
        continue;
      }
      if (isFull(true)) {
        return false;
      }
      final body = jsonEncode(p.toSharedAbUploadJson());
      final resp =
          await http.post(Uri.parse(api), headers: headers, body: body);
      final errMsg = _jsonDecodeActionResp(resp);
      if (errMsg.isNotEmpty) {
        BotToast.showText(contentColor: Colors.red, text: errMsg);
        return false;
      }
      addPeerWithoutPush(p);
    }
    return true;
  }

  @override
  Future<bool> changeTagForPeers(List<String> ids, List<dynamic> tags) async {
    final api =
        "${await bind.mainGetApiServer()}/api/ab/shared/peer/update/${profile.guid}";
    var headers = getHttpHeaders();
    headers['Content-Type'] = "application/json";
    var ret = true;
    for (var id in ids) {
      final body = jsonEncode({"id": id, "tags": tags});
      final resp = await http.put(Uri.parse(api), headers: headers, body: body);
      final errMsg = _jsonDecodeActionResp(resp);
      if (errMsg.isNotEmpty) {
        BotToast.showText(contentColor: Colors.red, text: errMsg);
        ret = false;
        break;
      }
      changeTagForPeersWithoutPush([id], tags);
    }
    return ret;
  }

  @override
  Future<bool> changeAlias({required String id, required String alias}) async {
    final api =
        "${await bind.mainGetApiServer()}/api/ab/shared/peer/update/${profile.guid}";
    var headers = getHttpHeaders();
    headers['Content-Type'] = "application/json";
    final body = jsonEncode({"id": id, "alias": alias});
    final resp = await http.put(Uri.parse(api), headers: headers, body: body);
    final errMsg = _jsonDecodeActionResp(resp);
    if (errMsg.isNotEmpty) {
      BotToast.showText(contentColor: Colors.red, text: errMsg);
      return false;
    }
    changeAliasWithoutPush(id: id, alias: alias);
    return true;
  }

  @override
  Future<bool> changeSharedPassword(String id, String password) async {
    final api =
        "${await bind.mainGetApiServer()}/api/ab/shared/peer/update/${profile.guid}";
    var headers = getHttpHeaders();
    headers['Content-Type'] = "application/json";
    final body = jsonEncode({"id": id, "password": password});
    final resp = await http.put(Uri.parse(api), headers: headers, body: body);
    final errMsg = _jsonDecodeActionResp(resp);
    if (errMsg.isNotEmpty) {
      BotToast.showText(contentColor: Colors.red, text: errMsg);
      return false;
    }
    pullAb(force: true, quiet: true);
    return true;
  }

  @override
  Future<void> syncFromRecent(List<Peer> recents, bool push) async {
    bool uiUpdate = false;
    bool peerSyncEqual(Peer a, Peer b) {
      return a.username == b.username &&
          a.platform == b.platform &&
          a.hostname == b.hostname;
    }

    Future<bool> syncOnePeer(Peer p, Peer r) async {
      p.username = r.username;
      p.hostname = r.hostname;
      p.platform = r.platform;
      final api =
          "${await bind.mainGetApiServer()}/api/ab/shared/peer/update/${profile.guid}";
      var headers = getHttpHeaders();
      headers['Content-Type'] = "application/json";
      final body = jsonEncode({
        "id": p.id,
        "username": r.username,
        "hostname": r.hostname,
        "platform": r.platform
      });
      final resp = await http.put(Uri.parse(api), headers: headers, body: body);
      final errMsg = _jsonDecodeActionResp(resp);
      if (errMsg.isNotEmpty) {
        debugPrint('errMsg: $errMsg');
        return false;
      }
      uiUpdate = true;
      return true;
    }

    final syncPeers = peers.where((p0) => p0.sameServer != true);
    for (var p in syncPeers) {
      Peer? r = recents.firstWhereOrNull((e) => e.id == p.id);
      if (r != null) {
        if (!peerSyncEqual(p, r)) {
          await syncOnePeer(p, r);
        }
      }
    }
    if (uiUpdate && gFFI.abModel.currentName.value == profile.name) {
      peers.refresh();
    }
  }

  @override
  Future<bool> deletePeers(List<String> ids) async {
    final api =
        "${await bind.mainGetApiServer()}/api/ab/shared/peer/${profile.guid}";
    var headers = getHttpHeaders();
    headers['Content-Type'] = "application/json";
    final body = jsonEncode(ids);
    final resp =
        await http.delete(Uri.parse(api), headers: headers, body: body);
    final errMsg = _jsonDecodeActionResp(resp);
    if (errMsg.isNotEmpty) {
      BotToast.showText(contentColor: Colors.red, text: errMsg);
      return false;
    }
    deletePeersWithoutPush(ids);
    return true;
  }
// #endregion

// #region Tags
  @override
  Future<bool> addTags(List<String> tagList) async {
    final api =
        "${await bind.mainGetApiServer()}/api/ab/shared/tag/add/${profile.guid}";
    var headers = getHttpHeaders();
    headers['Content-Type'] = "application/json";
    for (var t in tagList) {
      final body = jsonEncode({
        "name": t,
        "color": str2color2(t, existing: tagColors.values.toList()).value,
      });
      final resp =
          await http.post(Uri.parse(api), headers: headers, body: body);
      final errMsg = _jsonDecodeActionResp(resp);
      if (errMsg.isNotEmpty) {
        BotToast.showText(contentColor: Colors.red, text: errMsg);
        return false;
      }
      addTagsWithoutPush([t]);
    }
    return true;
  }

  @override
  Future<bool> renameTag(String oldTag, String newTag) async {
    if (tags.contains(newTag)) {
      BotToast.showText(
          contentColor: Colors.red, text: 'Tag $newTag already exists');
      return false;
    }
    final api =
        "${await bind.mainGetApiServer()}/api/ab/shared/tag/rename/${profile.guid}";
    var headers = getHttpHeaders();
    headers['Content-Type'] = "application/json";
    final body = jsonEncode({
      "old": oldTag,
      "new": newTag,
    });
    final resp = await http.put(Uri.parse(api), headers: headers, body: body);
    final errMsg = _jsonDecodeActionResp(resp);
    if (errMsg.isNotEmpty) {
      BotToast.showText(contentColor: Colors.red, text: errMsg);
      return false;
    }
    renameTagWithoutPush(oldTag, newTag);
    return true;
  }

  @override
  Future<bool> setTagColor(String tag, Color color) async {
    final api =
        "${await bind.mainGetApiServer()}/api/ab/shared/tag/update/${profile.guid}";
    var headers = getHttpHeaders();
    headers['Content-Type'] = "application/json";
    final body = jsonEncode({
      "name": tag,
      "color": color.value,
    });
    final resp = await http.put(Uri.parse(api), headers: headers, body: body);
    final errMsg = _jsonDecodeActionResp(resp);
    if (errMsg.isNotEmpty) {
      BotToast.showText(contentColor: Colors.red, text: errMsg);
      return false;
    }
    setTagColorWithoutPush(tag, color);
    return true;
  }

  @override
  Future<bool> deleteTag(String tag) async {
    final api =
        "${await bind.mainGetApiServer()}/api/ab/shared/tag/${profile.guid}";
    var headers = getHttpHeaders();
    headers['Content-Type'] = "application/json";
    final body = jsonEncode([tag]);
    final resp =
        await http.delete(Uri.parse(api), headers: headers, body: body);
    final errMsg = _jsonDecodeActionResp(resp);
    if (errMsg.isNotEmpty) {
      BotToast.showText(contentColor: Colors.red, text: errMsg);
      return false;
    }
    deleteTagWithoutPush(tag);
    return true;
  }

// #endregion
}

void _merge(Peer r, Peer p) {
  p.hash = r.hash.isEmpty ? p.hash : r.hash;
  p.username = r.username.isEmpty ? p.username : r.username;
  p.hostname = r.hostname.isEmpty ? p.hostname : r.hostname;
  p.platform = r.platform.isEmpty ? p.platform : r.platform;
  p.alias = p.alias.isEmpty ? r.alias : p.alias;
  p.forceAlwaysRelay = r.forceAlwaysRelay;
  p.rdpPort = r.rdpPort;
  p.rdpUsername = r.rdpUsername;
}

Map<String, dynamic> _jsonDecodeResp(String body, int statusCode) {
  try {
    Map<String, dynamic> json = jsonDecode(body);
    return json;
  } catch (e) {
    final err = body.isNotEmpty && body.length < 128 ? body : e.toString();
    if (statusCode != 200) {
      throw 'HTTP $statusCode, $err';
    }
    throw err;
  }
}

List<dynamic> _jsonDecodeRespList(String body, int statusCode) {
  try {
    List<dynamic> json = jsonDecode(body);
    return json;
  } catch (e) {
    final err = body.isNotEmpty && body.length < 128 ? body : e.toString();
    if (statusCode != 200) {
      throw 'HTTP $statusCode, $err';
    }
    throw err;
  }
}

String _jsonDecodeActionResp(http.Response resp) {
  var errMsg = '';
  if (resp.statusCode == 200 && resp.body.isEmpty) {
    // ok
  } else {
    try {
      errMsg = jsonDecode(resp.body)['error'].toString();
    } catch (_) {}
    if (errMsg.isEmpty) {
      if (resp.statusCode != 200) {
        errMsg = 'HTTP ${resp.statusCode}';
      }
      if (resp.body.isNotEmpty) {
        if (errMsg.isNotEmpty) {
          errMsg += ', ';
        }
        errMsg += resp.body;
      }
      if (errMsg.isEmpty) {
        errMsg = "unknown error";
      }
    }
  }
  return errMsg;
}
