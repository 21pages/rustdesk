import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'platform_model.dart';

class Peer {
  final String id;
  final String username;
  final String hostname;
  final String platform;
  String alias;
  List<dynamic> tags;
  bool forceAlwaysRelay = false;
  String rdpPort;
  String rdpUsername;
  late List<PortForward> portForwards;
  bool online = false;

  Peer.fromJson(Map<String, dynamic> json)
      : id = json['id'] ?? '',
        username = json['username'] ?? '',
        hostname = json['hostname'] ?? '',
        platform = json['platform'] ?? '',
        alias = json['alias'] ?? '',
        tags = json['tags'] ?? [],
        forceAlwaysRelay = json['forceAlwaysRelay'] == 'true',
        rdpPort = json['rdpPort'] ?? '',
        rdpUsername = json['rdpUsername'] ?? '' {
    try {
      portForwards = [];
      if (json['portForwards'] != null && json['portForwards'] is List) {
        portForwards = (json['portForwards'] as List)
            .map((e) => PortForward.fromJson(e))
            .toList();
      }
    } catch (e) {
      debugPrint('$e');
    }
  }

  Map<String, dynamic> toJson() {
    return <String, dynamic>{
      "id": id,
      "username": username,
      "hostname": hostname,
      "platform": platform,
      "alias": alias,
      "tags": tags,
      "forceAlwaysRelay": forceAlwaysRelay.toString(),
      "rdpPort": rdpPort,
      "rdpUsername": rdpUsername,
      "portForwards": portForwards,
    };
  }

  Peer({
    required this.id,
    required this.username,
    required this.hostname,
    required this.platform,
    required this.alias,
    required this.tags,
    required this.forceAlwaysRelay,
    required this.rdpPort,
    required this.rdpUsername,
    required this.portForwards,
  });

  Peer.loading()
      : this(
          id: '...',
          username: '...',
          hostname: '...',
          platform: '...',
          alias: '',
          tags: [],
          forceAlwaysRelay: false,
          rdpPort: '',
          rdpUsername: '',
          portForwards: [],
        );
}

class Peers extends ChangeNotifier {
  final String name;
  final String loadEvent;
  List<Peer> peers;
  static const _cbQueryOnlines = 'callback_query_onlines';

  Peers({required this.name, required this.peers, required this.loadEvent}) {
    platformFFI.registerEventHandler(_cbQueryOnlines, name, (evt) async {
      _updateOnlineState(evt);
    });
    platformFFI.registerEventHandler(loadEvent, name, (evt) async {
      _updatePeers(evt);
    });
  }

  @override
  void dispose() {
    platformFFI.unregisterEventHandler(_cbQueryOnlines, name);
    platformFFI.unregisterEventHandler(loadEvent, name);
    super.dispose();
  }

  Peer getByIndex(int index) {
    if (index < peers.length) {
      return peers[index];
    } else {
      return Peer.loading();
    }
  }

  int getPeersCount() {
    return peers.length;
  }

  void _updateOnlineState(Map<String, dynamic> evt) {
    evt['onlines'].split(',').forEach((online) {
      for (var i = 0; i < peers.length; i++) {
        if (peers[i].id == online) {
          peers[i].online = true;
        }
      }
    });

    evt['offlines'].split(',').forEach((offline) {
      for (var i = 0; i < peers.length; i++) {
        if (peers[i].id == offline) {
          peers[i].online = false;
        }
      }
    });

    notifyListeners();
  }

  void _updatePeers(Map<String, dynamic> evt) {
    final onlineStates = _getOnlineStates();
    peers = _decodePeers(evt['peers']);
    for (var peer in peers) {
      final state = onlineStates[peer.id];
      peer.online = state != null && state != false;
    }
    notifyListeners();
  }

  Map<String, bool> _getOnlineStates() {
    var onlineStates = <String, bool>{};
    for (var peer in peers) {
      onlineStates[peer.id] = peer.online;
    }
    return onlineStates;
  }

  List<Peer> _decodePeers(String peersStr) {
    try {
      if (peersStr == "") return [];
      List<dynamic> peers = json.decode(peersStr);
      return peers.map((peer) {
        return Peer.fromJson(peer as Map<String, dynamic>);
      }).toList();
    } catch (e) {
      debugPrint('peers(): $e');
    }
    return [];
  }
}

class PortForward {
  late int localPort;
  late String remoteHost;
  late int remotePort;

  PortForward(this.localPort, this.remoteHost, this.remotePort);

  PortForward.fromJson(List<dynamic> json) {
    try {
      if (json.length == 3 &&
          json[0] is int &&
          json[1] is String &&
          json[2] is int) {
        localPort = json[0] as int;
        remoteHost = json[1] as String;
        remotePort = json[2] as int;
      }
    } catch (e) {
      debugPrint('$e');
    }
  }
  List<dynamic> toJson() {
    return [localPort, remoteHost, remotePort].toList();
  }
}
