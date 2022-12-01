import 'dart:async';
import 'dart:convert';
import 'dart:html';

import 'package:flutter/material.dart';
import 'package:flutter_hbb/models/model.dart';
import 'package:flutter_hbb/models/peer_model.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:get/get.dart';
import 'package:http/http.dart' as http;

import '../common.dart';

class MyDeviceModel {
  var mdLoading = false.obs;
  var mdError = "".obs;
  var peers = List<Peer>.empty(growable: true).obs;

  var selectedTags = List<String>.empty(growable: true).obs;

  WeakReference<FFI> parent;

  MyDeviceModel(this.parent);

  Future<dynamic> mdPull() async {
    if (gFFI.userModel.userName.isEmpty) return;
    mdLoading.value = true;
    mdError.value = "";
    final api = "${await bind.mainGetApiServer()}/api/peers";
    try {
      final resp =
          await http.get(Uri.parse(api), headers: await getHttpHeaders());
      // print('')
      if (resp.body.isNotEmpty && resp.body.toLowerCase() != "null") {
        Map<String, dynamic> json = jsonDecode(resp.body);
        if (json.containsKey('error')) {
          mdError.value = json['error'];
        } else if (json.containsKey('data')) {
          final data = jsonDecode(json['data']);
          peers.clear();
          for (final peer in data['peers']) {
            peers.add(Peer.fromJson(peer));
          }
        }
        return resp.body;
      } else {
        return "";
      }
    } catch (err) {
      err.printError();
      mdError.value = err.toString();
    } finally {
      mdLoading.value = false;
    }
    return null;
  }

  void reset() {
    peers.clear();
  }

  void addPeer(Peer peer) {
    peers.removeWhere((e) => e.id == peer.id);
    peers.add(peer);
  }

  void changeTagForPeer(String id, List<dynamic> tags) {
    final it = peers.where((element) => element.id == id);
    if (it.isEmpty) {
      return;
    }
    it.first.tags = tags;
  }

  Future<void> push() async {
    if (gFFI.userModel.userName.isEmpty) return;
    mdLoading.value = true;
    final api = "${await bind.mainGetApiServer()}/api/ab";
    var authHeaders = await getHttpHeaders();
    authHeaders['Content-Type'] = "application/json";
    final peersJsonData = peers.map((e) => e.toJson()).toList();
    final body = jsonEncode({
      "data": jsonEncode({"peers": peersJsonData})
    });
    try {
      final resp =
          await http.post(Uri.parse(api), headers: authHeaders, body: body);
      mdError.value = "";
      await mdPull();
      debugPrint("resp: ${resp.body}");
    } catch (e) {
      mdError.value = e.toString();
    } finally {
      mdLoading.value = false;
    }
  }

  Peer? find(String id) {
    return peers.firstWhereOrNull((e) => e.id == id);
  }

  bool idContainBy(String id) {
    return peers.where((element) => element.id == id).isNotEmpty;
  }

  void deletePeer(String id) {
    peers.removeWhere((element) => element.id == id);
  }

  void unsetSelectedTags() {
    selectedTags.clear();
  }

  List<dynamic> getPeerTags(String id) {
    final it = peers.where((p0) => p0.id == id);
    if (it.isEmpty) {
      return [];
    } else {
      return it.first.tags;
    }
  }

  Future<void> setPeerAlias(String id, String value) async {
    final it = peers.where((p0) => p0.id == id);
    if (it.isNotEmpty) {
      it.first.alias = value;
      await push();
    }
  }

  Future<void> setPeerForceAlwaysRelay(String id, bool value) async {
    final it = peers.where((p0) => p0.id == id);
    if (it.isNotEmpty) {
      it.first.forceAlwaysRelay = value;
      await push();
    }
  }

  Future<void> setRdp(String id, String port, String username) async {
    final it = peers.where((p0) => p0.id == id);
    if (it.isNotEmpty) {
      it.first.rdpPort = port;
      it.first.rdpUsername = username;
      await push();
    }
  }

  Future<void> setPortForwards(Map<String, dynamic> evt) async {
    try {
      dynamic id = evt['id'];
      dynamic pfs = evt['pfs'];
      if (id is String && pfs is String) {
        final it = peers.where((p0) => p0.id == id);
        if (it.isNotEmpty) {
          List<dynamic> list = jsonDecode(pfs);
          List<PortForward> portForwards =
              list.map((item) => PortForward.fromJson(item)).toList();
          it.first.portForwards = portForwards;
          await push();
        }
      }
    } catch (e) {
      debugPrint('$e');
    }
  }

  void clear() {
    peers.clear();
  }
}
